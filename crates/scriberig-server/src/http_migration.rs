//! Maps browser migration-detail requests to the application boundary.

use scriberig_app::ApplicationContext;
use serde_json::{Value, json};

use crate::http::HttpState;
use crate::server::ApplicationService;

/// Returns canonical detail for one registered migration proposal.
pub(super) fn migration<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        (
            400,
            json!({"code": "invalid_request", "message": error.to_string()}),
        )
    })?;
    let migration_id = value
        .get("migrationId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            (
                400,
                json!({"code": "invalid_request", "message": "migrationId is required"}),
            )
        })?;
    let detail = state
        .service
        .migration_detail(&ApplicationContext::empty(), migration_id)
        .map_err(|error| {
            (
                if error.category() == "not_found" {
                    404
                } else {
                    400
                },
                json!({"code": error.category(), "message": error.to_string()}),
            )
        })?;
    let changes = detail
        .deltas
        .into_iter()
        .map(|delta| {
            let kind = if delta.before.is_none() {
                "created"
            } else if delta.after.is_none() {
                "removed"
            } else {
                "modified"
            };
            json!({
                "entityId": delta.entity.to_string(),
                "before": delta.before.map(|value| value.to_string()),
                "after": delta.after.map(|value| value.to_string()),
                "kind": kind,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "migrationId": detail.id,
        "base": detail.base.to_string(),
        "result": detail.result.to_string(),
        "source": "canonical",
        "changes": changes,
    }))
}
