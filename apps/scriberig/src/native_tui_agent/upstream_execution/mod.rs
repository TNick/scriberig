//! Runs upstream agent loops with `ScribeRig`-owned semantic and child tools.

mod broker;
mod delegation;
mod evidence;
mod execution;
mod lineage;
#[cfg(feature = "isolated-plugin-worker")]
mod plugin_execution;
pub(crate) use lineage::completed_root_context_fingerprint;
use lineage::provider_context_fingerprint;
#[cfg(feature = "isolated-plugin-worker")]
pub(super) use plugin_execution::{
    canonical_roots, include_runtime_loader_aliases, plugin_read_roots, protected_roots,
};

use std::sync::Arc;

use rpi_agent::{AgentError, AgentToolResult};
use scriberig_agent_runtime::ChildExecutionScope;
use scriberig_agent_runtime::upstream_invocation::{
    UpstreamBranchFailure, UpstreamBranchInvocation, UpstreamBranchResult,
};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationService, ToolExecutionEvidence,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

pub(super) const MAX_TOOL_ARGUMENT_BYTES: usize = 256 * 1024;
pub(super) const MAX_TOOL_OUTPUT_TEXT_BYTES: usize = 12_000;

/// Result of one recursively executed `ScribeRig` branch.
pub(super) struct UpstreamBranchExecution {
    /// Upstream final text and provider-neutral request projections.
    pub(crate) result: UpstreamBranchResult,
    /// `ScribeRig` tool evidence collected while dispatching branch tools.
    pub(crate) tool_evidence: Vec<ToolExecutionEvidence>,
    /// Opaque provider continuation kept in memory for this root execution.
    pub(crate) continuation: Option<rpi_ai::ProviderContinuation>,
}

/// Successful root execution plus the lineage fingerprint used for caching.
pub(crate) struct RootBranchRun {
    /// ScribeRig-compatible output and safe evidence.
    pub(crate) result: scriberig_agent_runtime::AgentResult,
    /// Opaque continuation extracted from a successful provider response.
    pub(crate) continuation: Option<rpi_ai::ProviderContinuation>,
}

/// Failure from a branch with any request and tool evidence already captured.
pub(crate) struct UpstreamBranchExecutionFailure {
    /// Safe upstream branch failure and captured model requests.
    pub(crate) failure: UpstreamBranchFailure,
    /// `ScribeRig` tool evidence collected before the failure.
    pub(crate) tool_evidence: Vec<ToolExecutionEvidence>,
}

/// Executes a root branch with explicit provider provenance and text streaming.
///
/// # Parameters
/// - `service`: Shared application service owning semantic and session state.
/// - `context`: Trusted root request context.
/// - `session`: Canonical `ScribeRig` session identity.
/// - `branch`: Canonical `ScribeRig` branch receiving the turn.
/// - `prompt`: User-authored instruction for this turn.
/// - `selection`: Exact selected workspace scope, when present.
/// - `profile`: Exact semantic profile captured for this root invocation.
/// - `project_store`: Durable project store for application tools.
/// - `provider`: Upstream provider bound to the selected model.
/// - `model`: Exact selected upstream model.
/// - `request_secret`: Optional destination-authorized request credential.
/// - `thinking_level`: Selected provider-supported thinking level.
/// - `cancelled`: `ScribeRig` operation cancellation flag.
/// - `provider_profile`: Safe provider profile used for provenance.
/// - `output_callback`: Optional provider-visible text observer.
///
/// # Returns
/// ScribeRig-compatible output, tool evidence, and request projections.
///
/// # Errors
/// Returns a provider, context, budget, or cancellation failure with captured
/// evidence retained for terminal persistence.
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_root_branch_with_options<B: ApplicationBackend + Send + Sync + 'static>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    branch: EntityId,
    prompt: &str,
    selection: Option<&scriberig_app::AgentSelection>,
    project_store: Arc<SqliteStore>,
    provider: Arc<dyn rpi_ai::Provider>,
    model: rpi_ai::Model,
    request_secret: Option<rpi_ai::RequestSecret>,
    thinking_level: Option<rpi_ai::ThinkingLevel>,
    cancelled: &std::sync::atomic::AtomicBool,
    provider_profile: Option<String>,
    output_callback: Option<scriberig_agent_runtime::OutputCallback>,
) -> Result<scriberig_agent_runtime::AgentResult, UpstreamBranchExecutionFailure> {
    execute_root_branch_with_continuation(
        service,
        context,
        session,
        branch,
        prompt,
        selection,
        None,
        project_store,
        provider,
        model,
        request_secret,
        thinking_level,
        cancelled,
        provider_profile,
        output_callback,
        None,
        None,
        None,
        None,
    )
    .map(|execution| execution.result)
}

#[expect(
    clippy::too_many_lines,
    reason = "This assembles the single upstream root-branch call with ScribeRig lineage, context, and continuation checks."
)]
/// Executes a root branch with a host-validated in-memory continuation.
///
/// # Parameters
/// - `service`: Shared application service owning semantic and session state.
/// - `context`: Trusted root request context.
/// - `session`: Canonical `ScribeRig` session identity.
/// - `branch`: Canonical `ScribeRig` branch receiving the turn.
/// - `prompt`: User-authored instruction for this turn.
/// - `selection`: Exact selected workspace scope, when present.
/// - `project_store`: Durable project store for application tools.
/// - `provider`: Upstream provider bound to the selected model.
/// - `model`: Exact selected upstream model.
/// - `request_secret`: Optional destination-authorized request credential.
/// - `thinking_level`: Selected provider-supported thinking level.
/// - `cancelled`: `ScribeRig` operation cancellation flag.
/// - `provider_profile`: Safe provider profile used for provenance.
/// - `output_callback`: Optional provider-visible text observer.
/// - `summary_callback`: Optional provider-visible reasoning-summary observer.
/// - `continuation`: Candidate provider handle and its prior context fingerprint.
/// - `provider_lifecycle`: Optional upstream lifecycle sink bound to the
///   catalog generation used by this invocation.
///
/// # Returns
/// The root result and extracted continuation.
///
/// # Errors
/// Returns a provider, context, budget, or cancellation failure with evidence.
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_root_branch_with_continuation<
    B: ApplicationBackend + Send + Sync + 'static,
>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    branch: EntityId,
    prompt: &str,
    selection: Option<&scriberig_app::AgentSelection>,
    profile: Option<&scriberig_app::AgentProfileDescriptor>,
    project_store: Arc<SqliteStore>,
    provider: Arc<dyn rpi_ai::Provider>,
    model: rpi_ai::Model,
    request_secret: Option<rpi_ai::RequestSecret>,
    thinking_level: Option<rpi_ai::ThinkingLevel>,
    cancelled: &std::sync::atomic::AtomicBool,
    provider_profile: Option<String>,
    output_callback: Option<scriberig_agent_runtime::OutputCallback>,
    summary_callback: Option<scriberig_agent_runtime::ReasoningSummaryCallback>,
    continuation: Option<(rpi_ai::ProviderContinuation, String)>,
    provider_lifecycle: Option<
        scriberig_agent_runtime::upstream_invocation::ProviderLifecycleContext,
    >,
    child_provider_resolver: Option<scriberig_agent_runtime::ChildProviderResolver>,
) -> Result<RootBranchRun, UpstreamBranchExecutionFailure> {
    let semantic_scope =
        selection.map(
            |selection| scriberig_agent_runtime::context::ProjectSemanticScope {
                project: selection
                    .project
                    .map_or_else(|| "unknown".to_owned(), |project| project.to_string()),
                snapshot: selection.snapshot.to_string(),
                root: selection.root.to_string(),
                selection: Some(selection.entity.to_string()),
            },
        );
    let runtime_context = selection.map_or_else(
        || context.clone(),
        |selection| context.clone().with_snapshot(selection.snapshot),
    );
    let compiled_context = match profile {
        Some(profile) => crate::agent_context::compile_branch_context_with_profile(
            service,
            &runtime_context,
            session,
            branch,
            prompt,
            semantic_scope.clone(),
            profile,
        ),
        None => crate::agent_context::compile_branch_context(
            service,
            &runtime_context,
            session,
            branch,
            prompt,
            semantic_scope.clone(),
        ),
    };
    let compiled = compiled_context.map_err(|_| UpstreamBranchExecutionFailure {
        failure: UpstreamBranchFailure {
            error: scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                "branch context compilation failed".to_owned(),
            ),
            model_requests: Vec::new(),
        },
        tool_evidence: Vec::new(),
    })?;
    let provider_context_fingerprint =
        provider_context_fingerprint(&compiled, &runtime_context.capabilities);
    let continuation = continuation.filter(|(handle, fingerprint)| {
        fingerprint == &provider_context_fingerprint
            && handle.is_scoped_to(provider.id(), &model.id)
            && provider.supports_response_continuation()
    });
    let invocation = UpstreamBranchInvocation {
        branch,
        provider,
        model,
        request_secret,
        bearer: true,
        applied_settings: None,
        provider_profile,
        thinking_level: thinking_level.unwrap_or_default(),
        context: compiled,
        tools: Vec::new(),
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
        provider_lifecycle,
    };
    let scope = ChildExecutionScope {
        session,
        parent_branch: branch,
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope,
        provider_resolver: child_provider_resolver,
    };
    let cancellation = CancellationToken::new();
    let run_cancellation = cancellation.clone();
    let run = execution::run_branch_with_semantic_broker(
        service,
        runtime_context,
        session,
        scope,
        project_store,
        invocation,
        prompt.to_owned(),
        run_cancellation,
        output_callback,
        summary_callback,
        continuation.map(|(handle, _)| handle),
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| UpstreamBranchExecutionFailure {
            failure: UpstreamBranchFailure {
                error: scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                    "upstream runtime could not start".to_owned(),
                ),
                model_requests: Vec::new(),
            },
            tool_evidence: Vec::new(),
        })?;
    let execution = runtime.block_on(async {
        tokio::pin!(run);
        loop {
            tokio::select! {
                result = &mut run => break result,
                () = tokio::time::sleep(std::time::Duration::from_millis(25)) => {
                    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                        cancellation.cancel();
                    }
                }
            }
        }
    })?;
    Ok(RootBranchRun {
        result: scriberig_agent_runtime::AgentResult {
            output: execution.result.output,
            tool_evidence: execution.tool_evidence,
            model_requests: execution.result.model_requests,
        },
        continuation: execution.continuation,
    })
}

/// Executes a plan worker using its already compiled `ScribeRig` context.
///
/// - `service`: Shared application boundary for semantic tools and children.
/// - `context`: Trusted context used to authorize worker operations.
/// - `session`: Canonical `ScribeRig` session identity.
/// - `branch`: Claimed durable worker branch.
/// - `prompt`: Exact worker purpose text.
/// - `project_store`: Durable project store for application tools.
/// - `invocation`: Upstream provider/model and compiled branch context.
/// - `semantic_scope`: Exact scope selected by the plan compiler.
/// - `cancelled`: `ScribeRig` operation cancellation flag.
///
/// Returns final output, tool evidence, and model request projections.
///
/// # Errors
/// Returns a provider, budget, or cancellation failure with captured evidence.
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_precompiled_branch<B: ApplicationBackend + Send + Sync + 'static>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    branch: EntityId,
    prompt: &str,
    project_store: Arc<SqliteStore>,
    invocation: UpstreamBranchInvocation,
    semantic_scope: Option<scriberig_agent_runtime::context::ProjectSemanticScope>,
    cancelled: &std::sync::atomic::AtomicBool,
    child_provider_resolver: Option<scriberig_agent_runtime::ChildProviderResolver>,
) -> Result<UpstreamBranchExecution, UpstreamBranchExecutionFailure> {
    let scope = ChildExecutionScope {
        session,
        parent_branch: branch,
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope,
        provider_resolver: child_provider_resolver,
    };
    let cancellation = CancellationToken::new();
    let run = execution::run_branch_with_semantic_broker(
        service,
        context.clone(),
        session,
        scope,
        project_store,
        invocation,
        prompt.to_owned(),
        cancellation.clone(),
        None,
        None,
        None,
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| UpstreamBranchExecutionFailure {
            failure: UpstreamBranchFailure {
                error: scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Agent(
                    "upstream runtime could not start".to_owned(),
                ),
                model_requests: Vec::new(),
            },
            tool_evidence: Vec::new(),
        })?;
    runtime.block_on(async {
        tokio::pin!(run);
        loop {
            tokio::select! {
                result = &mut run => break result,
                () = tokio::time::sleep(std::time::Duration::from_millis(25)) => {
                    if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                        cancellation.cancel();
                    }
                }
            }
        }
    })
}

/// Request sent from an upstream static tool adapter to its scoped host broker.
pub(super) struct ToolRequest {
    /// Trusted catalog name selected by upstream schema validation.
    pub(super) name: String,
    /// Validated JSON arguments.
    pub(super) arguments: serde_json::Value,
    /// Upstream cancellation signal for this tool call.
    pub(super) cancellation: CancellationToken,
    /// One-shot result channel back to the static tool adapter.
    pub(super) response: oneshot::Sender<Result<AgentToolResult, AgentError>>,
}
