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
        .find(|evidence| evidence_is_workspace_result(evidence))
        .and_then(|evidence| serde_json::from_slice::<serde_json::Value>(&evidence.output).ok())
    else {
        return Ok(None);
    };
    let manifest_id = result
        .tool_evidence
        .iter()
        .find(|evidence| evidence_is_scan_result(evidence))
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

/// Recognizes one successful tool result by its stable name or typed output.
///
/// The output type provides a fallback for runtime adapters that normalize the
/// provider-visible name while preserving the application result shape.
///
/// # Parameters
/// - `evidence`: Captured tool execution evidence.
/// - `object_type`: Expected application result type.
///
/// # Returns
/// Whether the evidence is successful and carries the expected result.
fn evidence_is_workspace_result(evidence: &scriberig_app::ToolExecutionEvidence) -> bool {
    evidence_is_type(evidence, "project_workspace")
        || output_summary(evidence)
            .is_some_and(|summary| summary.starts_with("active semantic workspace "))
        || output_workspace_handles(evidence).is_some()
}

/// Recognizes a successful project scan result by its stable type or summary.
///
/// # Parameters
/// - `evidence`: Captured tool execution evidence.
///
/// # Returns
/// Whether the evidence contains a persisted project scan manifest.
fn evidence_is_scan_result(evidence: &scriberig_app::ToolExecutionEvidence) -> bool {
    evidence_is_type(evidence, "project_scan")
        || output_summary(evidence)
            .is_some_and(|summary| summary.starts_with("scan persisted for "))
        || output_scan_handles(evidence).is_some()
}

/// Recognizes an exact project, snapshot, and root tuple in a tool result.
///
/// # Parameters
/// - `evidence`: Captured tool execution evidence.
///
/// # Returns
/// The parsed workspace identities when the output has that exact shape.
fn output_workspace_handles(
    evidence: &scriberig_app::ToolExecutionEvidence,
) -> Option<(
    scriberig_core::ProjectId,
    scriberig_core::SnapshotId,
    scriberig_core::EntityId,
)> {
    if evidence.failed {
        return None;
    }
    let output: serde_json::Value = serde_json::from_slice(&evidence.output).ok()?;
    let handles = output.get("handles")?.as_array()?;
    let [project, snapshot, root] = handles.as_slice() else {
        return None;
    };
    Some((
        project.as_str()?.parse().ok()?,
        snapshot.as_str()?.parse().ok()?,
        root.as_str()?.parse().ok()?,
    ))
}

/// Recognizes a persisted scan output with project and manifest identities.
///
/// # Parameters
/// - `evidence`: Captured tool execution evidence.
///
/// # Returns
/// Unit when the output contains a parseable project and manifest tuple.
fn output_scan_handles(evidence: &scriberig_app::ToolExecutionEvidence) -> Option<()> {
    if evidence.failed {
        return None;
    }
    let output: serde_json::Value = serde_json::from_slice(&evidence.output).ok()?;
    let handles = output.get("handles")?.as_array()?;
    let [project, manifest, ..] = handles.as_slice() else {
        return None;
    };
    project
        .as_str()?
        .parse::<scriberig_core::ProjectId>()
        .ok()?;
    manifest
        .as_str()?
        .parse::<scriberig_core::EntityId>()
        .ok()?;
    Some(())
}

/// Reads the safe summary from captured structured tool output.
///
/// # Parameters
/// - `evidence`: Captured tool execution evidence.
///
/// # Returns
/// The summary string when output contains valid structured JSON.
fn output_summary(evidence: &scriberig_app::ToolExecutionEvidence) -> Option<String> {
    if evidence.failed {
        return None;
    }
    serde_json::from_slice::<serde_json::Value>(&evidence.output)
        .ok()?
        .get("summary")?
        .as_str()
        .map(str::to_owned)
}

/// Recognizes one successful tool result by its stable name or typed output.
///
/// # Parameters
/// - `evidence`: Captured tool execution evidence.
/// - `object_type`: Expected application result type.
///
/// # Returns
/// Whether the evidence is successful and carries the expected result.
fn evidence_is_type(evidence: &scriberig_app::ToolExecutionEvidence, object_type: &str) -> bool {
    if evidence.failed {
        return false;
    }
    evidence.name == object_type
        || serde_json::from_slice::<serde_json::Value>(&evidence.output)
            .ok()
            .and_then(|output| output.get("object_type")?.as_str().map(str::to_owned))
            .is_some_and(|actual| actual == object_type)
}
