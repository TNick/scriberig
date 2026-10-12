//! Maps operation-segment HTTP requests onto the typed protocol dispatcher.

use serde_json::{Value, json};

use crate::dispatch::Dispatcher;
use crate::http::HttpState;
use crate::server::ApplicationService;

/// Dispatches one operation-segment request through the public protocol.
///
/// # Parameters
/// - state: Shared application service and fixture state.
/// - operation: Stable operation identifier from the URL.
/// - body: JSON request payload.
/// - `correlation_id`: Request correlation identity.
///
/// # Returns
/// A serialized typed protocol response envelope.
///
/// # Errors
/// Returns a transport status and structured body for malformed or unknown
/// operations.
pub(super) fn dispatch<S: ApplicationService>(
    state: &HttpState<S>,
    operation: &str,
    body: &str,
    correlation_id: String,
) -> Result<Value, (u16, Value)> {
    let request = decode_request(operation, body)?;
    if let scriberig_protocol::Request::Project(project) = request {
        return serde_json::to_value(super::project_routes::protocol(
            state,
            &project,
            correlation_id,
        ))
        .map_err(|error| {
            (
                500,
                json!({"code": "internal", "message": error.to_string()}),
            )
        });
    }
    let response = Dispatcher::new(&state.service).dispatch(scriberig_protocol::RequestEnvelope {
        version: scriberig_protocol::ProtocolVersion::default(),
        correlation_id,
        request,
    });
    serde_json::to_value(response).map_err(|error| {
        (
            500,
            json!({"code": "internal", "message": error.to_string()}),
        )
    })
}

/// Decodes one stable operation identifier into a protocol request variant.
///
/// # Parameters
/// - `operation`: Stable operation identifier from the URL.
/// - `body`: JSON request payload.
///
/// # Returns
/// The corresponding typed protocol request.
///
/// # Errors
/// Returns a transport status and structured body when decoding fails.
fn decode_request(
    operation: &str,
    body: &str,
) -> Result<scriberig_protocol::Request, (u16, Value)> {
    let invalid = |error: serde_json::Error| {
        (
            400,
            json!({"code": "invalid_request", "message": error.to_string()}),
        )
    };
    let request = match operation {
        "inspect" => {
            scriberig_protocol::Request::Inspect(serde_json::from_str(body).map_err(invalid)?)
        }
        "capabilities" => {
            scriberig_protocol::Request::Capabilities(serde_json::from_str(body).map_err(invalid)?)
        }
        "operation" | "operation_status" => scriberig_protocol::Request::OperationStatus(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "cancel" | "cancel_operation" => scriberig_protocol::Request::CancelOperation(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "subscribe" => {
            scriberig_protocol::Request::Subscribe(serde_json::from_str(body).map_err(invalid)?)
        }
        "query" => scriberig_protocol::Request::Query(serde_json::from_str(body).map_err(invalid)?),
        "list" => scriberig_protocol::Request::List(serde_json::from_str(body).map_err(invalid)?),
        "find" => scriberig_protocol::Request::Find(serde_json::from_str(body).map_err(invalid)?),
        "command" => {
            scriberig_protocol::Request::Command(serde_json::from_str(body).map_err(invalid)?)
        }
        "agent_start" => {
            scriberig_protocol::Request::AgentStart(serde_json::from_str(body).map_err(invalid)?)
        }
        "agent_continue" => {
            scriberig_protocol::Request::AgentContinue(serde_json::from_str(body).map_err(invalid)?)
        }
        "session" | "session_read" => {
            scriberig_protocol::Request::SessionRead(serde_json::from_str(body).map_err(invalid)?)
        }
        "project" => {
            scriberig_protocol::Request::Project(serde_json::from_str(body).map_err(invalid)?)
        }
        _ => {
            return Err((
                404,
                json!({
                    "code": "unsupported_operation",
                    "message": format!("unknown protocol operation: {operation}"),
                }),
            ));
        }
    };
    Ok(request)
}
