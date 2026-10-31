//! Creates and finalizes `ScribeRig` child branches for delegated work.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use rpi_agent::AgentError;
use scriberig_agent_runtime::context::{
    BranchExecutionContext, BranchRole, ContextCompiler, ContextPolicy,
};
use scriberig_agent_runtime::upstream_invocation::UpstreamBranchInvocation;
use scriberig_agent_runtime::{ChildExecutionScope, ChildRequest};
use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;
use tokio_util::sync::CancellationToken;

use super::{ToolRequest, execution};

/// Future returned by one recursive child invocation.
type ChildDispatchFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(String, Option<EntityId>), AgentError>> + Send + 'a>>;

/// Creates, recursively executes, and durably finalizes one child branch.
///
/// # Parameters
/// - `service`: Shared application service owning the session graph.
/// - `context`: Parent branch trusted context.
/// - `session`: Canonical `ScribeRig` session identity.
/// - `scope`: Parent branch identity and bounded recursion state.
/// - `invocation`: Parent provider/model template.
/// - `catalog`: Trusted provider-visible tool catalog.
/// - `request`: Validated delegate call.
/// - `project_store`: Shared durable project store.
/// - `cancellation`: Parent branch cancellation token.
/// - `output_callback`: Provider-visible text sink.
///
/// # Returns
/// Compact child result JSON and the durable child branch identity.
///
/// # Errors
/// Returns a safe failure after recording an abandoned child when possible.
#[allow(clippy::too_many_arguments)]
#[expect(
    clippy::too_many_lines,
    reason = "Child creation, context compilation, execution, and result settlement share one borrowed scope."
)]
pub(super) fn dispatch_child<'a, B: ApplicationBackend + Send + Sync + 'static + 'a>(
    service: &'a ApplicationService<B>,
    context: &'a ApplicationContext,
    session: EntityId,
    scope: &'a ChildExecutionScope,
    invocation: &'a UpstreamBranchInvocation,
    request: &'a ToolRequest,
    project_store: Arc<SqliteStore>,
    cancellation: CancellationToken,
    output_callback: Option<scriberig_agent_runtime::OutputCallback>,
) -> ChildDispatchFuture<'a> {
    Box::pin(async move {
        if scope.depth >= scriberig_agent_runtime::orchestration::RuntimeLimits::default().max_depth
        {
            return Err(AgentError::tool("branch depth limit reached"));
        }
        let child_request = child_request(request)?;
        let child_binding = match &child_request.execution {
            Some(selection) => Some(
                scope
                    .provider_resolver
                    .as_ref()
                    .ok_or_else(|| AgentError::tool("child model selection is unavailable"))?(
                    selection,
                )
                .map_err(|_| AgentError::tool("child model selection is unavailable"))?,
            ),
            None => None,
        };
        let child_handle = service
            .start_local_child_agent(
                session,
                scope.parent_branch,
                "delegated agent branch",
                format!(
                    "{}; expected result: {}",
                    child_request.purpose, child_request.expected_result
                ),
                child_request.prompt.clone(),
            )
            .map_err(|_| AgentError::tool("child branch could not be created"))?;
        let child_capabilities = crate::child_runtime::delegated_capabilities(
            &context.capabilities,
            &child_request.capabilities,
        );
        let child_context = ApplicationContext {
            capabilities: child_capabilities,
            ..context.clone()
        };
        let child_scope = ChildExecutionScope {
            session,
            parent_branch: child_handle.child_branch,
            depth: scope.depth + 1,
            active_branches: Arc::clone(&scope.active_branches),
            semantic_scope: scope.semantic_scope.clone(),
            provider_resolver: scope.provider_resolver.clone(),
        };
        let mut child_execution = BranchExecutionContext::new(
            session.to_string(),
            child_handle.child_branch.to_string(),
            &child_request.purpose,
            &child_request.prompt,
        )
        .with_branch_role(BranchRole::NonRoot {
            parent: scope.parent_branch.to_string(),
        });
        if let Some(semantic_scope) = child_scope.semantic_scope.clone() {
            child_execution = child_execution.with_project_scope(semantic_scope);
        }
        let compiled_context =
            ContextCompiler::new(ContextPolicy::default()).compile(&child_execution);
        let child_invocation = invocation_for_child(
            invocation,
            child_handle.child_branch,
            compiled_context,
            child_binding.as_ref(),
        );
        let child_result = execution::run_branch_with_semantic_broker(
            service,
            child_context,
            session,
            child_scope,
            project_store,
            child_invocation,
            child_request.prompt.clone(),
            cancellation,
            output_callback,
            None,
            None,
        )
        .await;
        match child_result {
            Ok(execution) => {
                let compact = scriberig_agent_runtime::AgentResult {
                    output: execution.result.output.clone(),
                    tool_evidence: execution.tool_evidence.clone(),
                    model_requests: execution.result.model_requests.clone(),
                };
                let mut result_handles = crate::child_runtime::compact_result_handles(&compact);
                result_handles.push(format!("branch:{}", child_handle.child_branch));
                let finalized = service
                    .complete_local_child_agent_with_evidence(
                        &child_handle,
                        execution.result.output.clone(),
                        result_handles.clone(),
                        execution.tool_evidence,
                        execution.result.model_requests,
                    )
                    .map_err(|_| AgentError::tool("child branch could not be finalized"))?;
                let handles = result_handles
                    .into_iter()
                    .chain([format!("branch_result:{}", finalized.branch_result)])
                    .collect::<Vec<_>>();
                Ok((
                    serde_json::json!({
                        "summary": finalized.final_output,
                        "branch": child_handle.child_branch,
                        "branch_result": finalized.branch_result,
                        "handles": handles,
                    })
                    .to_string(),
                    Some(child_handle.child_branch),
                ))
            }
            Err(failure) => {
                let _ = service.fail_local_child_agent_with_evidence(
                    &child_handle,
                    "child agent execution failed",
                    failure.failure.model_requests,
                    failure.tool_evidence,
                );
                Err(AgentError::tool("child branch execution failed"))
            }
        }
    })
}

/// Parses a validated delegate call into the durable child request model.
///
/// # Parameters
/// - `request`: Provider tool call already validated against the branch schema.
///
/// # Returns
/// The narrowed child purpose, prompt, capabilities, and expected result.
///
/// # Errors
/// Returns a tool error when any required argument has the wrong shape.
fn child_request(request: &ToolRequest) -> Result<ChildRequest, AgentError> {
    Ok(ChildRequest {
        purpose: request.arguments["purpose"]
            .as_str()
            .ok_or_else(|| AgentError::tool("invalid delegation purpose"))?
            .to_owned(),
        prompt: request.arguments["prompt"]
            .as_str()
            .ok_or_else(|| AgentError::tool("invalid delegation prompt"))?
            .to_owned(),
        capabilities: request.arguments["capabilities"]
            .as_array()
            .ok_or_else(|| AgentError::tool("invalid delegated capabilities"))?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| AgentError::tool("invalid delegated capability"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        expected_result: request.arguments["expected_result"]
            .as_str()
            .ok_or_else(|| AgentError::tool("invalid expected result"))?
            .to_owned(),
        execution: parse_child_execution(request.arguments.get("execution"))?,
    })
}

/// Parses the optional upstream model assignment requested for a child.
///
/// # Parameters
/// - `value`: Optional provider-visible execution object.
///
/// # Returns
/// The explicit provider/model assignment, when supplied.
///
/// # Errors
/// Returns a tool error when the assignment is malformed or incomplete.
fn parse_child_execution(
    value: Option<&serde_json::Value>,
) -> Result<Option<scriberig_session::ModelAssignment>, AgentError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let provider_id = value["provider_id"]
        .as_str()
        .ok_or_else(|| AgentError::tool("invalid child provider selection"))?;
    let model_id = value["model_id"]
        .as_str()
        .ok_or_else(|| AgentError::tool("invalid child model selection"))?;
    let thinking_level = value
        .get("thinking_level")
        .filter(|level| !level.is_null())
        .map(|level| {
            level
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| AgentError::tool("invalid child thinking selection"))
        })
        .transpose()?;
    Ok(Some(scriberig_session::ModelAssignment {
        provider_id: provider_id.to_owned(),
        model_id: model_id.to_owned(),
        thinking_level,
    }))
}

/// Copies exact parent provider selection onto a fresh child branch.
///
/// # Parameters
/// - `parent`: Parent invocation providing the selected provider/model policy.
/// - `branch`: Durable `ScribeRig` child branch identity.
/// - `context`: `ScribeRig`-compiled child context.
/// - `binding`: Optional resolved provider/model binding for the child.
///
/// # Returns
/// A new branch invocation with no parent branch tools and its own branch
/// activity sequence, because durable activity order is per branch.
pub(super) fn invocation_for_child(
    parent: &UpstreamBranchInvocation,
    branch: EntityId,
    context: scriberig_agent_runtime::context::CompiledContext,
    binding: Option<&scriberig_agent_runtime::ChildProviderBinding>,
) -> UpstreamBranchInvocation {
    let provider = binding.as_ref().map_or_else(
        || Arc::clone(&parent.provider),
        |binding| Arc::clone(&binding.provider),
    );
    let model = binding
        .as_ref()
        .map_or_else(|| parent.model.clone(), |binding| binding.model.clone());
    let request_secret = binding.as_ref().map_or_else(
        || parent.request_secret.clone(),
        |binding| binding.request_secret.clone(),
    );
    let bearer = binding
        .as_ref()
        .map_or(parent.bearer, |binding| binding.bearer);
    let provider_profile = binding.as_ref().map_or_else(
        || parent.provider_profile.clone(),
        |binding| binding.provider_profile.clone(),
    );
    let thinking_level = binding
        .as_ref()
        .map_or(parent.thinking_level, |binding| binding.thinking_level);
    UpstreamBranchInvocation {
        branch,
        provider,
        model,
        request_secret,
        bearer,
        applied_settings: None,
        provider_profile,
        thinking_level,
        context,
        tools: Vec::new(),
        activity: Arc::new(scriberig_agent_runtime::orchestration::ActivitySequence::default()),
        provider_lifecycle: parent.provider_lifecycle.clone(),
    }
}
/// Creates a branch invocation template without its parent-specific tool set.
///
/// The template stays on the same branch, so it shares that branch's activity
/// sequence with the model loop it feeds.
///
/// # Parameters
/// - `invocation`: Invocation whose provider policy is inherited by children.
///
/// # Returns
/// Provider/model/security values with an empty tool set.
pub(super) fn invocation_without_tools(
    invocation: &UpstreamBranchInvocation,
) -> UpstreamBranchInvocation {
    UpstreamBranchInvocation {
        activity: Arc::clone(&invocation.activity),
        provider_lifecycle: None,
        ..invocation_for_child(
            invocation,
            invocation.branch,
            invocation.context.clone(),
            None,
        )
    }
}
