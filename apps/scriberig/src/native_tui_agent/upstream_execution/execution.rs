//! Coordinates one upstream model loop with the scoped `ScribeRig` tool broker.

use std::sync::{Arc, Mutex};

use rpi_agent::AgentError;
use scriberig_agent_runtime::ChildExecutionScope;
use scriberig_agent_runtime::tools::{ToolCatalog, ToolDescriptor};
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchFailure, UpstreamBranchInvocation,
    run_upstream_branch_invocation_with_continuation,
};
use scriberig_agent_runtime::upstream_tool::SemanticAgentTool;
use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::child_runtime::inspect_catalog;

use super::{
    ToolRequest, UpstreamBranchExecution, UpstreamBranchExecutionFailure, broker, delegation,
};

/// Executes one branch and recursively owns model-requesting child branches.
///
/// Upstream owns provider streaming and tool-call validation. The scoped
/// broker owns `ScribeRig` authorization, dispatch, and durable child results.
///
/// # Parameters
/// - `service`: Shared application service owning branch and operation state.
/// - `context`: Trusted authorization context for this branch.
/// - `session`: Canonical `ScribeRig` session identity.
/// - `scope`: Canonical `ScribeRig` branch scope and recursion budgets.
/// - `project_store`: Durable project store supplied to application tools.
/// - `invocation`: Exact upstream provider/model/context for this branch.
/// - `prompt`: User or parent-tool instruction for the branch.
/// - `cancellation`: Branch-local cancellation token.
/// - `output_callback`: Optional provider-visible text sink.
/// - `summary_callback`: Optional provider-visible reasoning-summary sink.
///
/// # Returns
/// Final output, captured model requests, and executed tool evidence.
///
/// # Errors
/// Returns upstream failures together with evidence captured before settlement.
#[allow(clippy::too_many_arguments)] // Keeps the shared broker call contract stable.
pub(super) async fn run_branch_with_semantic_broker<'a, B>(
    service: &'a ApplicationService<B>,
    context: ApplicationContext,
    session: EntityId,
    scope: ChildExecutionScope,
    project_store: Arc<SqliteStore>,
    invocation: UpstreamBranchInvocation,
    prompt: String,
    cancellation: CancellationToken,
    output_callback: Option<scriberig_agent_runtime::OutputCallback>,
    summary_callback: Option<scriberig_agent_runtime::ReasoningSummaryCallback>,
    continuation: Option<rpi_ai::ProviderContinuation>,
) -> Result<UpstreamBranchExecution, UpstreamBranchExecutionFailure>
where
    B: ApplicationBackend + Sync + 'a,
{
    if invocation.branch != scope.parent_branch || scope.session != session {
        return Err(failure(
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::InvalidToolSet,
            Vec::new(),
        ));
    }
    let Some(_active_branch) = ActiveBranchGuard::acquire(&scope) else {
        return Err(failure(
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                "active branch limit reached".to_owned(),
            ),
            Vec::new(),
        ));
    };

    let catalog = inspect_catalog();
    let capabilities = context.capabilities.clone();
    let descriptors = catalog.descriptors_for(&capabilities);
    let (sender, receiver) = mpsc::unbounded_channel();
    let evidence = Arc::new(Mutex::new(Vec::new()));
    let tools = build_tools(
        &descriptors,
        &catalog,
        &capabilities,
        invocation.branch,
        &sender,
    )?;
    drop(sender);
    let broker_invocation = delegation::invocation_without_tools(&invocation);
    let branch_invocation = UpstreamBranchInvocation {
        tools,
        ..invocation
    };
    let broker_evidence = Arc::clone(&evidence);
    let broker_run = broker::dispatch_tools(
        service,
        &context,
        session,
        scope.clone(),
        project_store,
        broker_invocation,
        descriptors,
        receiver,
        broker_evidence,
        cancellation.clone(),
        output_callback.clone(),
    );
    let model_run = run_upstream_branch_invocation_with_continuation(
        branch_invocation,
        prompt,
        cancellation,
        output_callback,
        summary_callback,
        continuation,
    );
    let (result, broker_result) = tokio::join!(model_run, broker_run);
    let tool_evidence = evidence
        .lock()
        .map(|records| records.clone())
        .unwrap_or_default();
    if let Err(error) = broker_result {
        let model_requests = result.map_or_else(
            |failure| failure.model_requests,
            |success| success.0.model_requests,
        );
        return Err(UpstreamBranchExecutionFailure {
            failure: UpstreamBranchFailure {
                error: scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                    error,
                ),
                model_requests,
            },
            tool_evidence,
        });
    }
    match result {
        Ok((result, continuation)) => Ok(UpstreamBranchExecution {
            result,
            tool_evidence,
            continuation,
        }),
        Err(failure) => Err(UpstreamBranchExecutionFailure {
            failure,
            tool_evidence,
        }),
    }
}

/// Builds static upstream tools that forward calls to this branch's broker.
///
/// # Parameters
/// - `descriptors`: Capability-filtered tool descriptors.
/// - `catalog`: Trusted catalog used for tool authorization.
/// - `capabilities`: Immutable branch capability snapshot.
/// - `branch`: Canonical `ScribeRig` branch identity.
/// - `sender`: Owned channel captured by upstream `'static` tool adapters.
///
/// This boundary intentionally accepts only `ToolCatalog` descriptors. Do not
/// merge `rpi-extensions` registrations here: native plugins run in process at
/// the pinned API and can bypass `ScribeRig`'s broker through ambient access.
/// The host currently rejects their activation before they can reach this
/// broker; keep this type restriction when an isolated upstream worker arrives.
///
/// # Returns
/// Upstream adapters for every visible descriptor.
///
/// # Errors
/// Returns a safe failure when a descriptor cannot be adapted.
fn build_tools(
    descriptors: &[ToolDescriptor],
    catalog: &ToolCatalog,
    capabilities: &scriberig_app::CapabilityContext,
    branch: EntityId,
    sender: &mpsc::UnboundedSender<ToolRequest>,
) -> Result<Vec<Arc<SemanticAgentTool>>, UpstreamBranchExecutionFailure> {
    descriptors
        .iter()
        .map(|descriptor| {
            let sender = sender.clone();
            let name = descriptor.definition.name.clone();
            SemanticAgentTool::new(
                &descriptor.definition,
                descriptor
                    .invocation_label
                    .clone()
                    .unwrap_or_else(|| descriptor.definition.name.clone()),
                branch,
                catalog.clone(),
                capabilities.clone(),
                move |_id, arguments, cancellation, _on_update| {
                    let sender = sender.clone();
                    let name = name.clone();
                    async move {
                        let (response, receiver) = tokio::sync::oneshot::channel();
                        sender
                            .send(ToolRequest {
                                name,
                                arguments,
                                cancellation,
                                response,
                            })
                            .map_err(|_| AgentError::tool("semantic tool broker closed"))?;
                        receiver
                            .await
                            .map_err(|_| AgentError::tool("semantic tool broker closed"))?
                    }
                },
            )
            .map(Arc::new)
            .map_err(|_| {
                failure(
                scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::InvalidToolSet,
                Vec::new(),
            )
            })
        })
        .collect()
}

/// Builds a branch failure with no captured tool evidence.
///
/// - `error`: Safe upstream error category and message.
/// - `model_requests`: Provider-neutral requests captured before the failure.
///
/// # Returns
/// The branch failure value.
fn failure(
    error: scriberig_agent_runtime::upstream_invocation::UpstreamBranchError,
    model_requests: Vec<scriberig_session::ModelRequestProjection>,
) -> UpstreamBranchExecutionFailure {
    UpstreamBranchExecutionFailure {
        failure: UpstreamBranchFailure {
            error,
            model_requests,
        },
        tool_evidence: Vec::new(),
    }
}

/// Reserves an active-branch slot and releases it when this loop settles.
struct ActiveBranchGuard<'a> {
    /// Shared recursion counter owned by the branch scope.
    scope: &'a ChildExecutionScope,
}

impl<'a> ActiveBranchGuard<'a> {
    /// Acquires a slot while respecting the configured active-branch budget.
    ///
    /// - `scope`: Shared active-branch counter.
    ///
    /// # Returns
    /// A guard when budget permits another branch, or `None` at the limit.
    fn acquire(scope: &'a ChildExecutionScope) -> Option<Self> {
        let limit =
            scriberig_agent_runtime::orchestration::RuntimeLimits::default().max_active_branches;
        let mut active = scope
            .active_branches
            .load(std::sync::atomic::Ordering::Acquire);
        loop {
            if active >= limit {
                return None;
            }
            match scope.active_branches.compare_exchange_weak(
                active,
                active + 1,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            ) {
                Ok(_) => return Some(Self { scope }),
                Err(observed) => active = observed,
            }
        }
    }
}

impl Drop for ActiveBranchGuard<'_> {
    /// Releases the active-branch slot.
    fn drop(&mut self) {
        self.scope
            .active_branches
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}
