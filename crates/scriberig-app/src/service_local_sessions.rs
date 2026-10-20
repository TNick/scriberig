//! Provides local session lifecycle operations for the application service.

use super::ApplicationService;
use scriberig_session::{
    Branch, CacheDisposition, ExecutionActivity, ModelInvocation, ModelRequestProjection,
    ToolInvocation, ToolResult,
};

#[path = "service_local_sessions/operations.rs"]
mod operations;
#[path = "service_local_sessions/start_agent.rs"]
mod start_agent;

impl<B> ApplicationService<B> {
    /// Records safe model requests on a coordinator-owned worker branch.
    ///
    /// The caller must include the mutated branch in its normal session save.
    /// This shares the same invocation-ID and activity projection used by
    /// root and child agent completion operations.
    ///
    /// - `self`: Application identity allocator.
    /// - `branch`: Exact claimed worker branch receiving request history.
    /// - `model_requests`: Bounded provider-neutral requests captured at runtime.
    pub fn persist_plan_worker_model_requests(
        &self,
        branch: &mut Branch,
        model_requests: Vec<ModelRequestProjection>,
    ) {
        self.persist_model_requests(branch, model_requests);
    }

    /// Records safe model requests on a session branch before its terminal result.
    ///
    /// - `self`: Application identity allocator.
    /// - `branch`: Exact root, child, or worker branch receiving request history.
    /// - `model_requests`: Bounded provider-neutral requests captured by the runtime.
    pub(crate) fn persist_model_requests(
        &self,
        branch: &mut Branch,
        model_requests: Vec<ModelRequestProjection>,
    ) {
        persist_execution_evidence(self, branch, Vec::new(), model_requests);
    }
}

/// Persists ordered model requests and tool invocation/result pairs.
///
/// - `service`: Application identity allocator.
/// - `branch`: Session branch that owns the execution evidence.
/// - `tool_evidence`: Ordered tool inputs and bounded outputs from the runtime.
/// - `model_requests`: Ordered provider-neutral request projections.
fn persist_execution_evidence<B>(
    service: &ApplicationService<B>,
    branch: &mut Branch,
    tool_evidence: Vec<crate::ToolExecutionEvidence>,
    model_requests: Vec<ModelRequestProjection>,
) {
    let mut ordered = tool_evidence
        .into_iter()
        .map(|item| (item.ordinal, ExecutionEvidence::Tool(item)))
        .chain(
            model_requests
                .into_iter()
                .map(|item| (item.ordinal, ExecutionEvidence::Model(item))),
        )
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(ordinal, _)| *ordinal);
    let mut parent_model_invocation = None;
    for (_, evidence) in ordered {
        match evidence {
            ExecutionEvidence::Tool(evidence) => {
                let invocation = service.next_entity(0xB7);
                let result = service.next_entity(0xB8);
                branch.tool_invocations.push(ToolInvocation {
                    id: invocation,
                    name: evidence.name,
                    input: evidence.input,
                    parent_model_invocation,
                    presentation_input: evidence.presentation_input,
                    presentation_label: evidence.presentation_label,
                    category: evidence.category,
                });
                if let Some(child_branch) = evidence.child_branch {
                    if let Some(activity) = branch
                        .activity
                        .iter_mut()
                        .find(|activity| **activity == ExecutionActivity::ChildBranch(child_branch))
                    {
                        *activity = ExecutionActivity::ChildBranchSpawned {
                            branch: child_branch,
                            parent_tool_invocation: invocation,
                        };
                    }
                }
                branch.tool_results.push(ToolResult {
                    id: result,
                    invocation,
                    output: evidence.output,
                    failed: evidence.failed,
                    presentation_preview: None,
                });
                branch
                    .activity
                    .push(ExecutionActivity::ToolInvocation(invocation));
                branch.activity.push(ExecutionActivity::ToolResult(result));
            }
            ExecutionEvidence::Model(request) => {
                parent_model_invocation = Some(persist_model_request(service, branch, request));
            }
        }
    }
}

/// Persists one request under its stable model-invocation activity identity.
///
/// - `service`: Application identity allocator.
/// - `branch`: Branch that owns the invocation.
/// - `request`: Bounded provider-neutral request projection.
fn persist_model_request<B>(
    service: &ApplicationService<B>,
    branch: &mut Branch,
    request: ModelRequestProjection,
) -> scriberig_core::EntityId {
    let invocation = service.next_entity(0xB6);
    branch.model_invocations.push(ModelInvocation {
        id: invocation,
        branch: branch.id,
        ordinal: request.ordinal,
        agent_profile: request
            .profile
            .as_ref()
            .map(|profile| format!("{}@{}", profile.identity, profile.revision)),
        context_fingerprint: request.context_fingerprint.clone(),
        provider_profile: request.provider_profile.clone(),
        provider_id: request.provider_id.clone(),
        model: Some(request.model.clone()),
        model_profile: None,
        provider_model_id: request.provider_id.as_ref().map(|_| request.model.clone()),
        thinking_level: request.thinking_level.clone(),
        request_fingerprint: request.request_fingerprint.clone(),
        cache: CacheDisposition::NotUsed,
        output: None,
        request: Some(request),
    });
    branch
        .activity
        .push(ExecutionActivity::ModelInvocation(invocation));
    invocation
}

/// Distinguishes provider requests from tool calls while preserving order.
enum ExecutionEvidence {
    /// Safe normalized model request.
    Model(ModelRequestProjection),
    /// Executed tool and bounded result evidence.
    Tool(crate::ToolExecutionEvidence),
}
