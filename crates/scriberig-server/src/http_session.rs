//! Implements the direct browser session-read route.

use scriberig_app::{ApplicationContext, CapabilityContext, CapabilityId};
use serde_json::{Value, json};

use crate::http::HttpState;
use crate::server::ApplicationService;

/// Reads one durable session through the browser transport route.
///
/// # Parameters
/// - `state`: HTTP application state.
/// - `body`: JSON object containing a session identity.
///
/// # Returns
/// The direct session response expected by the browser client.
///
/// # Errors
/// Returns a structured HTTP error for malformed or unavailable sessions.
pub(super) fn read<S: ApplicationService>(
    state: &HttpState<S>,
    body: &str,
) -> Result<Value, (u16, Value)> {
    let value: Value = serde_json::from_str(body).map_err(|error| {
        (
            400,
            json!({"code": "invalid_request", "message": error.to_string()}),
        )
    })?;
    let session = value
        .get("session")
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            (
                400,
                json!({"code": "invalid_request", "message": "missing session id"}),
            )
        })?;
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("session capability")
        ]),
        ..ApplicationContext::empty()
    };
    let result = match state.service.read_session(&context, session) {
        Ok(result) => serde_json::to_value(result),
        Err(error) if matches!(error.category(), "not_found" | "invalid_request") => {
            serde_json::to_value(empty_session(session))
        }
        Err(error) => {
            return Err((
                404,
                json!({"code": error.category(), "message": error.to_string()}),
            ));
        }
    };
    result.map_err(|error| {
        (
            500,
            json!({"code": "internal", "message": error.to_string()}),
        )
    })
}

/// Creates the empty session shape used before a browser session is persisted.
///
/// # Parameters
/// - `session`: Browser-owned session identity.
///
/// # Returns
/// An empty durable-session response with the requested identity.
fn empty_session(session: &str) -> scriberig_protocol::SessionReadResponse {
    scriberig_protocol::SessionReadResponse {
        session: scriberig_protocol::SessionReference {
            id: session.to_owned(),
        },
        branches: Vec::new(),
        execution: scriberig_protocol::SessionExecutionSelection::default(),
    }
}
