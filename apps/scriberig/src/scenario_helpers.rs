//! Builds upstream message values for `ScribeRig` semantic scenarios.

use rpi_ai::types::{Context, Message, UserContent};
use rpi_ai::{AssistantMessage, Content, Model, StopReason};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TOOL_CALL_ID: AtomicU64 = AtomicU64::new(1);

/// Returns the latest user-authored text in an upstream context.
///
/// # Parameters
/// - `context`: Provider context supplied by `rpi-agent`.
///
/// # Returns
/// The most recent user text, or an empty string when none is present.
pub(crate) fn user_prompt(context: &Context) -> String {
    context
        .messages
        .iter()
        .rev()
        .find_map(|message| match message {
            Message::User(message) => match &message.content {
                UserContent::Text(text) => Some(text.clone()),
                UserContent::Blocks(blocks) => Some(text_content(blocks)),
            },
            _ => None,
        })
        .unwrap_or_default()
}

/// Returns text from prior tool results in provider message order.
///
/// # Parameters
/// - `context`: Provider context supplied by `rpi-agent`.
///
/// # Returns
/// Text blocks from tool results, joined within each message.
pub(crate) fn tool_messages(context: &Context) -> Vec<String> {
    context
        .messages
        .iter()
        .filter_map(|message| match message {
            Message::ToolResult(result) => {
                let mut text = text_content(&result.content);
                if let Some(details) = result
                    .details
                    .as_ref()
                    .and_then(serde_json::Value::as_object)
                {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&serde_json::Value::Object(details.clone()).to_string());
                }
                Some(text)
            }
            _ => None,
        })
        .collect()
}

/// Creates a normal upstream final assistant message.
///
/// # Parameters
/// - `model`: Exact upstream model bound to the scenario.
/// - `text`: Provider-visible final text.
///
/// # Returns
/// An upstream message with a stop finish reason.
pub(crate) fn final_response(model: &Model, text: &str) -> AssistantMessage {
    let mut message = AssistantMessage::empty(
        model.api.clone(),
        model.provider.clone(),
        model.id.clone(),
        0,
    );
    message.content.push(Content::text(text));
    message.stop_reason = StopReason::Stop;
    message
}

/// Creates an upstream assistant message requesting one semantic tool.
///
/// # Parameters
/// - `model`: Exact upstream model bound to the scenario.
/// - `name`: `ScribeRig` semantic tool name.
/// - `arguments`: Structured tool arguments.
///
/// # Returns
/// An upstream message with one tool call and a tool-use finish reason.
pub(crate) fn tool_call(
    model: &Model,
    name: &str,
    arguments: serde_json::Value,
) -> AssistantMessage {
    let mut message = AssistantMessage::empty(
        model.api.clone(),
        model.provider.clone(),
        model.id.clone(),
        0,
    );
    message
        .content
        .push(Content::ToolCall(rpi_ai::types::ToolCall {
            kind: rpi_ai::types::ToolCallType,
            id: format!(
                "scenario-{name}-{}",
                NEXT_TOOL_CALL_ID.fetch_add(1, Ordering::Relaxed)
            ),
            name: name.to_owned(),
            arguments,
            thought_signature: None,
            namespace: None,
        }));
    message.stop_reason = StopReason::ToolUse;
    message
}

/// Creates an upstream terminal error message for invalid scenario state.
///
/// # Parameters
/// - `model`: Exact upstream model bound to the scenario.
/// - `message`: Safe scenario failure explanation.
///
/// # Returns
/// An upstream message marked with an error finish reason.
pub(crate) fn error_response(model: &Model, message: String) -> AssistantMessage {
    AssistantMessage::terminal(
        model.api.clone(),
        model.provider.clone(),
        model.id.clone(),
        StopReason::Error,
        message,
        0,
    )
}

/// Joins text blocks while dropping non-text details from scenario inspection.
///
/// # Parameters
/// - `blocks`: Provider content blocks in message order.
///
/// # Returns
/// Text blocks separated by newlines.
fn text_content(blocks: &[Content]) -> String {
    blocks
        .iter()
        .filter_map(|block| match block {
            Content::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{tool_call, tool_messages};
    use rpi_ai::types::{Context, Message, ToolResultMessage, ToolResultRole};
    use rpi_ai::{Api, Content, Model};

    /// Ensures repeated scenario calls use distinct provider tool-call IDs.
    #[test]
    fn repeated_tool_calls_have_unique_ids() {
        let model = Model::new(
            "test-model",
            "Test model",
            Api::AnthropicMessages,
            "test-provider",
            "https://example.test",
        );
        let first = tool_call(&model, "semantic_find", serde_json::json!({}));
        let second = tool_call(&model, "semantic_find", serde_json::json!({}));
        let id = |message: &rpi_ai::AssistantMessage| match &message.content[0] {
            Content::ToolCall(call) => call.id.clone(),
            _ => panic!("scenario response should contain a tool call"),
        };

        assert_ne!(id(&first), id(&second));
    }

    /// Keeps semantic result handles available to deterministic provider flows.
    #[test]
    fn tool_messages_include_structured_result_details() {
        let context = Context::new(vec![Message::ToolResult(Box::new(ToolResultMessage {
            role: ToolResultRole,
            tool_call_id: "call-1".to_owned(),
            tool_name: "semantic_render".to_owned(),
            content: vec![Content::text("rendered source")],
            details: Some(serde_json::json!({"handles":["source_digest:abc"]})),
            usage: None,
            added_tool_names: Vec::new(),
            is_error: false,
            timestamp: 0,
        }))]);

        let messages = tool_messages(&context);

        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("rendered source"));
        assert!(messages[0].contains("source_digest:abc"));
    }
}
