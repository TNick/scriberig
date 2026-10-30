//! Adapts provider aggregate edits to the public project candidate operation.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use serde_json::Value;
use std::sync::Arc;

use scriberig_project::{ProjectService, ProjectSourceChange, ProjectSourceChangeKind};
use scriberig_store_sqlite::SqliteStore;

/// Submits provider source effects through the public shared project service.
///
/// - `store`: Shared project store.
/// - `project`: Trusted selected project.
/// - `entity`: Selected semantic entity retained in the result.
/// - `snapshot`: Trusted exact base snapshot.
/// - `effects`: Provider source effect array.
///
/// Tool result containing the ordinary durable candidate identity.
///
/// # Returns
/// The created candidate identity and selected exact snapshot.
///
/// # Errors
/// Returns a tool error when proposal or candidate creation fails.
pub(super) fn propose(
    store: &Arc<SqliteStore>,
    project: ProjectId,
    entity: EntityId,
    snapshot: SnapshotId,
    effects: &Value,
) -> Result<ToolResult, ToolError> {
    let changes = effects
        .as_array()
        .ok_or_else(|| tool_error("proposal effects must be an array"))?
        .iter()
        .map(parse_effect)
        .collect::<Result<Vec<_>, _>>()?;
    let candidate = ProjectService::new(Arc::clone(store))
        .propose_project_change(project, snapshot, changes)
        .map_err(|error| tool_error(&error.to_string()))?;
    let candidate_id = candidate.id.clone();
    Ok(ToolResult {
        entity,
        revision: None,
        snapshot: Some(snapshot),
        object_type: Some("migration_candidate".to_owned()),
        summary: Some("project candidate created".to_owned()),
        handles: vec![candidate_id],
    })
}

/// Parses one provider effect into the project service's typed operation.
///
/// - `effect`: Provider-supplied JSON effect object.
///
/// A validated project source change.
///
/// # Returns
/// A typed project source change when the provider effect is valid.
///
/// # Errors
/// Returns a safe application error for invalid fields.
fn parse_effect(effect: &Value) -> Result<ProjectSourceChange, ToolError> {
    let path = effect
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| tool_error("proposal effect path is missing"))?;
    let kind = match effect.get("kind").and_then(Value::as_str) {
        Some("create") => ProjectSourceChangeKind::Create,
        Some("update") => ProjectSourceChangeKind::Update,
        Some("delete") => ProjectSourceChangeKind::Delete,
        _ => return Err(tool_error("proposal effect kind is invalid")),
    };
    let content = match effect.get("content") {
        Some(Value::String(text)) => Some(text.as_bytes().to_vec()),
        Some(Value::Array(bytes)) => Some(
            bytes
                .iter()
                .map(|byte| byte.as_u64().and_then(|number| u8::try_from(number).ok()))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| tool_error("proposal effect bytes are invalid"))?,
        ),
        Some(Value::Null) | None => None,
        _ => return Err(tool_error("proposal effect content is invalid")),
    };
    Ok(ProjectSourceChange {
        path: path.to_owned(),
        kind,
        content,
    })
}

/// Creates a safe project proposal error.
///
/// - `message`: Bounded non-secret diagnostic.
///
/// A normalized semantic tool error.
/// # Returns
/// A bounded project-candidate tool error.
fn tool_error(message: &str) -> ToolError {
    ToolError::Application {
        category: "project_candidate".to_owned(),
        message: message.to_owned(),
    }
}
