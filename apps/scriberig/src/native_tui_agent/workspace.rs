//! Resolves the active project workspace from agent tool evidence.

use scriberig_agent_runtime::AgentMessage;
use scriberig_app::ApplicationError;

/// Extracts the exact workspace returned by the project workspace tool.
///
/// # Parameters
/// - `result`: Completed runtime transcript.
/// - `super`: Parent module containing the native runner.
///
/// # Returns
/// The returned project workspace when the agent activated one.
///
/// # Errors
/// Returns an internal error when a workspace tool result omits valid references.
pub(super) fn workspace_from_result(
    result: &scriberig_agent_runtime::AgentResult,
) -> Result<Option<scriberig_tui::AgentWorkspace>, ApplicationError> {
    let Some(tool) = result
        .messages
        .iter()
        .rev()
        .find_map(|message| match message {
            AgentMessage::Tool(tool)
                if tool.object_type.as_deref() == Some("project_workspace") =>
            {
                Some(tool)
            }
            _ => None,
        })
    else {
        return Ok(None);
    };
    let manifest_id = result
        .messages
        .iter()
        .find_map(|message| match message {
            AgentMessage::Tool(tool) if tool.object_type.as_deref() == Some("project_scan") => {
                tool.handles.get(1).cloned()
            }
            _ => None,
        })
        .ok_or_else(|| ApplicationError::Internal {
            message: "project workspace has no reviewed manifest reference".into(),
        })?;
    let [project, snapshot, root] = tool.handles.as_slice() else {
        return Err(ApplicationError::Internal {
            message: "project workspace result must contain project, snapshot, and root".into(),
        });
    };
    let parse_error = |message: &str| ApplicationError::Internal {
        message: message.to_owned(),
    };
    let project = project
        .parse()
        .map_err(|error| parse_error(&format!("invalid workspace project: {error}")))?;
    let snapshot = snapshot
        .parse()
        .map_err(|error| parse_error(&format!("invalid workspace snapshot: {error}")))?;
    let root = root
        .parse()
        .map_err(|error| parse_error(&format!("invalid workspace root: {error}")))?;
    let name = tool
        .summary
        .as_deref()
        .and_then(|summary| summary.strip_prefix("active semantic workspace "))
        .and_then(|summary| summary.split_once(" at snapshot ").map(|(name, _)| name))
        .ok_or_else(|| parse_error("project workspace result omitted its name"))?;
    Ok(Some(scriberig_tui::AgentWorkspace {
        project,
        name: name.to_owned(),
        snapshot,
        root,
        manifest_id,
    }))
}
