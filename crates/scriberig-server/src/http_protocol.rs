//! Maps operation-segment HTTP requests onto the typed protocol dispatcher.

use serde_json::{Value, json};

use crate::dispatch::Dispatcher;
use crate::http::HttpState;
use crate::server::ApplicationService;

/// Dispatches one operation-segment request through the public protocol.
///
/// - state: Shared application service and fixture state.
/// - operation: Stable operation identifier from the URL.
/// - body: JSON request payload.
/// - `correlation_id`: Request correlation identity.
///
/// A serialized typed protocol response envelope.
///
/// Returns a transport status and structured body for malformed or unknown
/// operations.
pub(super) fn dispatch<S: ApplicationService + 'static>(
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
    let lifecycle_handler = state.plan_lifecycle_handler.as_ref().map(|handler| {
        let handler = std::sync::Arc::clone(handler);
        std::sync::Arc::new(
            move |service: &&S,
                  context: &scriberig_app::ApplicationContext,
                  request: &scriberig_app::PlanLifecycleRequest,
                  action: scriberig_app::PlanLifecycleAction| {
                handler(*service, context, request, action)
            },
        ) as std::sync::Arc<crate::http::PlanLifecycleHandler<&S>>
    });
    let response = Dispatcher::for_principal(&state.service, state.principal.clone())
        .dispatch_with_plan_lifecycle(
            scriberig_protocol::RequestEnvelope {
                version: scriberig_protocol::ProtocolVersion::default(),
                correlation_id,
                request,
            },
            lifecycle_handler.as_deref(),
        );
    serde_json::to_value(response).map_err(|error| {
        (
            500,
            json!({"code": "internal", "message": error.to_string()}),
        )
    })
}

/// Decodes one stable operation identifier into a protocol request variant.
///
/// - `operation`: Stable operation identifier from the URL.
/// - `body`: JSON request payload.
///
/// The corresponding typed protocol request.
///
/// Returns a transport status and structured body when decoding fails.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
        "agent_new_session" => scriberig_protocol::Request::AgentNewSession(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "agent_continue" => {
            scriberig_protocol::Request::AgentContinue(serde_json::from_str(body).map_err(invalid)?)
        }
        "agent_continue_at" => scriberig_protocol::Request::AgentContinueAt(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "session" | "session_read" => {
            scriberig_protocol::Request::SessionRead(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_create" => {
            scriberig_protocol::Request::PlanCreate(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_lifecycle" => {
            scriberig_protocol::Request::PlanLifecycle(serde_json::from_str(body).map_err(invalid)?)
        }
        "plan_assignment" => scriberig_protocol::Request::PlanAssignment(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "plan_replan" => {
            scriberig_protocol::Request::PlanReplan(serde_json::from_str(body).map_err(invalid)?)
        }
        "model_request_detail" => scriberig_protocol::Request::ModelRequestDetail(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "tool_result_detail" => scriberig_protocol::Request::ToolResultDetail(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "model_library" => {
            scriberig_protocol::Request::ModelLibrary(serde_json::from_str(body).map_err(invalid)?)
        }
        "agent_presentation_preferences" => {
            scriberig_protocol::Request::AgentPresentationPreferencesRead(
                serde_json::from_str(body).map_err(invalid)?,
            )
        }
        "set_agent_presentation_preferences" => {
            scriberig_protocol::Request::AgentPresentationPreferencesSet(
                serde_json::from_str(body).map_err(invalid)?,
            )
        }
        "session_model_selection" => scriberig_protocol::Request::SessionModelSelection(
            serde_json::from_str(body).map_err(invalid)?,
        ),
        "session_thinking_selection" => scriberig_protocol::Request::SessionThinkingSelection(
            serde_json::from_str(body).map_err(invalid)?,
        ),
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

#[cfg(test)]
mod tests {
    use super::decode_request;

    /// Verifies HTTP operations decode model-library and selection requests.
    #[test]
    fn model_library_and_selection_operations_decode() {
        assert!(matches!(
            decode_request("model_library", "{}"),
            Ok(scriberig_protocol::Request::ModelLibrary(_))
        ));
        assert!(matches!(
            decode_request(
                "model_request_detail",
                r#"{"session":{"id":"session-1"},"branch":{"id":"branch-1"},"invocation":"invocation-1"}"#
            ),
            Ok(scriberig_protocol::Request::ModelRequestDetail(_))
        ));
        assert!(matches!(
            decode_request(
                "tool_result_detail",
                r#"{"session":{"id":"session-1"},"branch":{"id":"branch-1"},"result":"result-1"}"#
            ),
            Ok(scriberig_protocol::Request::ToolResultDetail(_))
        ));
        assert!(matches!(
            decode_request(
                "session_model_selection",
                r#"{"session":{"id":"session-1"},"expected_revision":2,"model_profile":"profile-1"}"#
            ),
            Ok(scriberig_protocol::Request::SessionModelSelection(_))
        ));
        assert!(matches!(
            decode_request(
                "session_thinking_selection",
                r#"{"session":{"id":"session-1"},"expected_revision":3,"thinking_level":"deep-dive"}"#
            ),
            Ok(scriberig_protocol::Request::SessionThinkingSelection(_))
        ));
    }
}
