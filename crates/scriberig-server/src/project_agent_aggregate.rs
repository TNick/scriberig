//! Adapts provider aggregate edits to the public project candidate operation.

use scriberig_agent_core::{ToolError, ToolResult};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use serde_json::{Value, json};

use crate::http::HttpState;
use crate::http::project_routes::propose_change;
use crate::server::ApplicationService;

/// Submits provider source effects through the public shared project service.
///
/// - `state`: Shared HTTP application state.
/// - `project`: Trusted selected project.
/// - `entity`: Selected semantic entity retained in the result.
/// - `snapshot`: Trusted exact base snapshot.
/// - `effects`: Provider source effect array.
///
/// Tool result containing the ordinary durable candidate identity.
///
/// Returns a tool error when proposal or candidate creation fails.
pub(super) fn propose<S: ApplicationService>(
    state: &HttpState<S>,
    project: ProjectId,
    entity: EntityId,
    snapshot: SnapshotId,
    effects: &Value,
) -> Result<ToolResult, ToolError> {
    let result = propose_change(
        state,
        &json!({
            "project": project.to_string(),
            "base_snapshot": snapshot.to_string(),
            "effects": effects,
        })
        .to_string(),
    )
    .map_err(|(_, value)| ToolError::Application {
        category: "project_candidate".to_owned(),
        message: value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("candidate failed")
            .to_owned(),
    })?;
    let candidate = result
        .get("candidate")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::Application {
            category: "project_candidate".to_owned(),
            message: "candidate identity missing".to_owned(),
        })?;
    Ok(ToolResult {
        entity,
        revision: None,
        snapshot: Some(snapshot),
        object_type: Some("migration_candidate".to_owned()),
        summary: Some("project candidate created".to_owned()),
        handles: vec![candidate.to_owned()],
    })
}
