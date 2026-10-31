//! Decodes browser application commands through the shared server boundary.

use serde_json::{Value, json};

use crate::server::ApplicationService;

use super::HttpState;
use scriberig_app::ApplicationContext;

/// Executes one declared application command for browser clients.
pub(super) fn command<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        (
            400,
            json!({"code": "invalid_request", "message": error.to_string()}),
        )
    })?;
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let payload = value.get("payload").unwrap_or(&Value::Null);
    let operation = state
        .service
        .command(&ApplicationContext::empty(), kind, payload)
        .map_err(|error| {
            (
                400,
                json!({"code": error.category(), "message": error.to_string()}),
            )
        })?;
    serde_json::to_value(operation).map_err(|error| {
        (
            500,
            json!({"code": "internal", "message": error.to_string()}),
        )
    })
}
