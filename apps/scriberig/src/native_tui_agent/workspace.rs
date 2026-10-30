//! Resolves the active project workspace from agent tool evidence.

use scriberig_app::ApplicationError;

/// Extracts the exact workspace returned by the project workspace tool.
///
/// - `result`: Completed runtime transcript.
/// - `super`: Parent module containing the native runner.
///
/// The returned project workspace when the agent activated one.
///
/// Returns an internal error when a workspace tool result omits valid references.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn workspace_from_result(
    result: &scriberig_agent_runtime::AgentResult,
) -> Result<Option<scriberig_tui::AgentWorkspace>, ApplicationError> {
    let Some(workspace) = result
        .tool_evidence
        .iter()
        .rev()
        .find(|evidence| evidence.name == "project_workspace" && !evidence.failed)
        .and_then(|evidence| serde_json::from_slice::<serde_json::Value>(&evidence.output).ok())
    else {
        return Ok(None);
    };
    let manifest_id = result
        .tool_evidence
        .iter()
        .find(|evidence| evidence.name == "project_scan" && !evidence.failed)
        .and_then(|evidence| serde_json::from_slice::<serde_json::Value>(&evidence.output).ok())
        .and_then(|output| {
            output
                .get("handles")?
                .as_array()?
                .get(1)?
                .as_str()
                .map(str::to_owned)
        })
        .ok_or_else(|| ApplicationError::Internal {
            message: "project workspace has no reviewed manifest reference".into(),
        })?;
    let handles = workspace
        .get("handles")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ApplicationError::Internal {
            message: "project workspace result omitted its handles".into(),
        })?;
    let [project, snapshot, root] = handles.as_slice() else {
        return Err(ApplicationError::Internal {
            message: "project workspace result must contain project, snapshot, and root".into(),
        });
    };
    let parse_error = |message: &str| ApplicationError::Internal {
        message: message.to_owned(),
    };
    let project = project
        .as_str()
        .ok_or_else(|| parse_error("invalid workspace project handle"))?
        .parse()
        .map_err(|error| parse_error(&format!("invalid workspace project: {error}")))?;
    let snapshot = snapshot
        .as_str()
        .ok_or_else(|| parse_error("invalid workspace snapshot handle"))?
        .parse()
        .map_err(|error| parse_error(&format!("invalid workspace snapshot: {error}")))?;
    let root = root
        .as_str()
        .ok_or_else(|| parse_error("invalid workspace root handle"))?
        .parse()
        .map_err(|error| parse_error(&format!("invalid workspace root: {error}")))?;
    let name = workspace
        .get("summary")
        .and_then(serde_json::Value::as_str)
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
