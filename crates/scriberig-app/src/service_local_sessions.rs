//! Provides local session lifecycle operations for the application service.

use super::ApplicationService;
use scriberig_session::{Branch, ExecutionActivity, ToolInvocation, ToolResult};

#[path = "service_local_sessions/operations.rs"]
mod operations;
#[path = "service_local_sessions/start_agent.rs"]
mod start_agent;

/// Persists tool invocation/result pairs in their owning durable branch.
///
/// # Parameters
/// - `service`: Application identity allocator.
/// - `branch`: Session branch that owns the execution evidence.
/// - `evidence`: Ordered tool inputs and bounded outputs from the runtime.
fn persist_tool_evidence<B>(
    service: &ApplicationService<B>,
    branch: &mut Branch,
    evidence: Vec<crate::ToolExecutionEvidence>,
) {
    for evidence in evidence {
        let invocation = service.next_entity(0xB7);
        let result = service.next_entity(0xB8);
        branch.tool_invocations.push(ToolInvocation {
            id: invocation,
            name: evidence.name,
            input: evidence.input,
        });
        branch.tool_results.push(ToolResult {
            id: result,
            invocation,
            output: evidence.output,
            failed: evidence.failed,
        });
        branch
            .activity
            .push(ExecutionActivity::ToolInvocation(invocation));
        branch.activity.push(ExecutionActivity::ToolResult(result));
    }
}
