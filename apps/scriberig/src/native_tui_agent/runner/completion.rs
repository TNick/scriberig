//! Persists successful upstream branch results as `ScribeRig` activity.

use scriberig_app::{ApplicationBackend, ApplicationError, ApplicationService};

use super::super::{selection_handles, workspace_from_result};

/// Persists the successful root-branch result and returns its TUI handle.
///
/// - `service`: Application service owning durable agent activity.
/// - `handle`: Accepted operation and root branch identity.
/// - `session`: Session that owns the completed branch.
/// - `selection`: Workspace selection used to build the execution context.
/// - `result`: Completed agent output and safe execution evidence.
///
/// # Returns
/// The stable operation, session, branch, and workspace handles.
///
/// # Errors
/// Returns an error when result projection or durable completion fails.
pub(super) fn complete_accepted_result<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    handle: &scriberig_app::LocalAgentHandle,
    session: scriberig_core::EntityId,
    selection: Option<&scriberig_app::AgentSelection>,
    result: scriberig_agent_runtime::AgentResult,
) -> Result<scriberig_tui::AgentHandle, ApplicationError> {
    let workspace = workspace_from_result(&result)?;
    let mut result_handles = crate::child_runtime::compact_result_handles(&result);
    let workspace_handles = workspace.as_ref().map_or_else(
        || selection_handles(selection),
        |workspace| {
            vec![
                format!("project:{}", workspace.project),
                format!("snapshot:{}", workspace.snapshot),
                format!("root:{}", workspace.root),
                format!("manifest:{}", workspace.manifest_id),
            ]
        },
    );
    for handle in workspace_handles {
        if !result_handles.contains(&handle) {
            result_handles.push(handle);
        }
    }
    service.complete_local_agent_with_evidence(
        handle,
        result.output,
        result_handles,
        result.tool_evidence,
        result.model_requests,
    )?;
    Ok(scriberig_tui::AgentHandle {
        operation: handle.operation.clone(),
        session,
        branch: Some(handle.root_branch),
        workspace,
        failure: None,
    })
}
