//! Tests the OpenAI-compatible wire conversion and response normalization.

use super::{WireRequest, WireResponse};
use crate::provider::{FinishReason, ModelMessage, ModelRequest, ToolDefinition};

/// Verifies provider-neutral tool definitions serialize to function tools.
#[test]
fn request_serialization_contains_model_messages_and_tools() {
    let mut request = ModelRequest::new("model", vec![ModelMessage::user("hello")]);
    request.tools.push(ToolDefinition {
        name: "inspect".to_owned(),
        description: "Inspect an entity".to_owned(),
        parameters: serde_json::json!({"type": "object"}),
    });

    let wire = serde_json::to_value(WireRequest::from_request(&request, false)).expect("wire JSON");

    assert_eq!(wire["model"], "model");
    assert_eq!(wire["messages"][0]["role"], "user");
    assert_eq!(wire["tools"][0]["function"]["name"], "inspect");
    let streaming = serde_json::to_value(WireRequest::from_request(&request, true))
        .expect("streaming wire JSON");
    assert_eq!(streaming["stream"], true);
}

/// Verifies fragmented provider function arguments are parsed only when complete.
#[test]
fn response_normalization_parses_structured_tool_arguments() {
    let wire: WireResponse = serde_json::from_value(serde_json::json!({
        "model": "model",
        "choices": [{
            "message": {
                "content": null,
                "tool_calls": [{
                    "id": "call-1",
                    "function": {
                        "name": "inspect",
                        "arguments": r#"{"entity":"ent_1"}"#
                    }
                }]
            },
            "finish_reason": "tool_calls"
        }]
    }))
    .expect("wire response");

    let response = wire
        .into_response(Some("request-1".to_owned()))
        .expect("response");

    assert_eq!(response.finish_reason, FinishReason::ToolCall);
    assert_eq!(response.tool_calls[0].arguments["entity"], "ent_1");
    assert_eq!(response.metadata.request_id.as_deref(), Some("request-1"));
}
