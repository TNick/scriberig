//! Builds upstream message values for `ScribeRig` semantic scenarios.

use rpi_ai::types::{Context, Message, UserContent};
use rpi_ai::{AssistantMessage, Content, Model, StopReason};

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
            Message::ToolResult(result) => Some(text_content(&result.content)),
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
            id: format!("scenario-{name}"),
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
