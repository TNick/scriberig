//! Captures bounded safe request data before provider-specific serialization.

use crate::provider::ModelRequest;

const MAX_CAPTURED_CONTENT_CHARS: usize = 15_000;

/// Projects provider-neutral request fields without transport metadata.
///
/// - `request`: Normalized request immediately before adapter execution.
///
/// A bounded request projection containing messages and trusted tool schemas.
/// # Returns
/// The operation result.
pub(super) fn capture(request: &ModelRequest) -> scriberig_session::ModelRequestProjection {
    let mut budget = MAX_CAPTURED_CONTENT_CHARS;
    let mut truncated = false;
    let mut messages = Vec::new();
    for message in &request.messages {
        if budget == 0 {
            truncated = true;
            break;
        }
        let content = take_bounded(&message.content, &mut budget);
        truncated |= content.chars().count() < message.content.chars().count();
        let role = match message.role {
            crate::provider::MessageRole::System => "system",
            crate::provider::MessageRole::User => "user",
            crate::provider::MessageRole::Assistant => "assistant",
            crate::provider::MessageRole::Tool => "tool",
        };
        messages.push(scriberig_session::ModelRequestMessage {
            role: role.to_owned(),
            content,
        });
    }
    let mut tools = Vec::new();
    for tool in &request.tools {
        if budget == 0 {
            truncated = true;
            break;
        }
        let name = take_bounded(&tool.name, &mut budget);
        let description = take_bounded(&tool.description, &mut budget);
        truncated |= name.len() < tool.name.len() || description.len() < tool.description.len();
        let encoded_schema = serde_json::to_string(&tool.parameters).unwrap_or_default();
        let parameters = if encoded_schema.chars().count() <= budget {
            budget -= encoded_schema.chars().count();
            Some(encoded_schema)
        } else {
            truncated = true;
            None
        };
        tools.push(scriberig_session::ModelRequestTool {
            name,
            description,
            parameters,
        });
    }
    scriberig_session::ModelRequestProjection {
        ordinal: 0,
        model: request.model.clone(),
        thinking_level: request.thinking_level.clone(),
        messages,
        tools,
        truncated,
    }
}

/// Copies at most the remaining character budget from one string.
///
/// - `value`: Content to copy.
/// - `budget`: Mutable remaining character allowance.
///
/// The bounded prefix and updated remaining budget.
/// # Returns
/// The operation result.
fn take_bounded(value: &str, budget: &mut usize) -> String {
    let bounded = value.chars().take(*budget).collect::<String>();
    *budget -= bounded.chars().count();
    bounded
}

#[cfg(test)]
mod tests {
    use super::capture;
    use crate::provider::{ModelMessage, ModelRequest, ToolChoice};

    /// Omits transport-only authorization metadata from public request detail.
    #[test]
    fn authorization_marker_is_absent_from_captured_projection() {
        let mut metadata = std::collections::BTreeMap::new();
        metadata.insert(
            "authorization".to_owned(),
            "FAKE_PROVIDER_SECRET_MARKER_8391".to_owned(),
        );
        let request = ModelRequest {
            model: "model-a".to_owned(),
            thinking_level: Some("careful".to_owned()),
            messages: vec![ModelMessage::user("safe prompt")],
            tools: Vec::new(),
            tool_choice: ToolChoice::Auto,
            response_schema: None,
            temperature: None,
            max_tokens: None,
            metadata,
        };

        let projection = capture(&request);
        let encoded = serde_json::to_string(&projection).expect("safe projection JSON");

        assert!(!encoded.contains("FAKE_PROVIDER_SECRET_MARKER_8391"));
        assert!(!encoded.contains("authorization"));
        assert!(encoded.contains("safe prompt"));
    }

    /// Marks captured prompt detail when its provider-neutral budget truncates.
    #[test]
    fn captured_model_request_detail_is_bounded_and_marks_truncation() {
        let request = ModelRequest::new("model-a", vec![ModelMessage::user("x".repeat(20_000))]);

        let projection = capture(&request);

        assert!(projection.truncated);
        assert_eq!(projection.messages[0].content.chars().count(), 15_000);
    }
}
