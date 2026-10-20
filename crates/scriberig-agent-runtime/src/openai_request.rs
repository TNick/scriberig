//! Serializes normalized model requests into OpenAI-compatible fields.

use serde::Serialize;

use crate::provider::{MessageRole, ModelRequest, ToolChoice};
/// Serializes a provider-neutral request into the compatible wire subset.
#[derive(Serialize)]
pub(super) struct WireRequest {
    model: String,
    messages: Vec<WireMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<WireTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "is_false")]
    stream: bool,
}

impl WireRequest {
    /// Converts a request to provider wire fields without local reasoning maps.
    ///
    /// - `request`: Provider-neutral request.
    /// - `stream`: Whether the provider should stream SSE fragments.
    ///
    /// OpenAI-compatible request fields.
    /// # Returns
    /// The operation result.
    pub(super) fn from_request(request: &ModelRequest, stream: bool) -> Self {
        let tool_choice = match &request.tool_choice {
            ToolChoice::Auto => Some(serde_json::json!("auto")),
            ToolChoice::None => Some(serde_json::json!("none")),
            ToolChoice::Required(name) => Some(serde_json::json!({
                "type": "function",
                "function": {"name": name},
            })),
        };
        let response_format = request.response_schema.as_ref().map(|schema| {
            serde_json::json!({
                "type": "json_schema",
                "json_schema": {"name": "scriberig_output", "schema": schema},
            })
        });
        Self {
            model: request.model.clone(),
            messages: request
                .messages
                .iter()
                .map(WireMessage::from_message)
                .collect(),
            tools: request
                .tools
                .iter()
                .map(WireTool::from_definition)
                .collect(),
            tool_choice,
            response_format,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream,
        }
    }
}

/// Omits the default false streaming flag from ordinary requests.
#[allow(clippy::trivially_copy_pass_by_ref)]
///
/// - `value`: Streaming flag to inspect.
///
/// `true` when the flag is false.
/// # Returns
/// The operation result.
fn is_false(value: &bool) -> bool {
    !*value
}

/// Serializes one provider-neutral message.
#[derive(Serialize)]
struct WireMessage {
    role: &'static str,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

impl WireMessage {
    /// Converts a semantic message role to its wire spelling.
    ///
    /// - `message`: Provider-neutral message.
    ///
    /// Wire-compatible message fields.
    /// # Returns
    /// The operation result.
    fn from_message(message: &crate::provider::ModelMessage) -> Self {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        Self {
            role,
            content: message.content.clone(),
            tool_call_id: message.tool_call_id.clone(),
        }
    }
}

#[derive(Serialize)]
/// Represents one tool descriptor in an OpenAI-compatible request.
struct WireTool {
    r#type: &'static str,
    function: WireFunction,
}

#[derive(Serialize)]
/// Represents a callable function descriptor in a model request.
struct WireFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

impl WireTool {
    ///
    /// - `definition`: Trusted provider-neutral tool definition.
    ///
    /// Wire-compatible function tool fields.
    /// # Returns
    /// The operation result.
    fn from_definition(definition: &crate::provider::ToolDefinition) -> Self {
        Self {
            r#type: "function",
            function: WireFunction {
                name: definition.name.clone(),
                description: definition.description.clone(),
                parameters: provider_schema(&definition.parameters),
            },
        }
    }
}

/// Removes Scriberig-only sensitivity annotations from provider schemas.
///
/// - `schema`: Trusted schema used for local presentation redaction.
///
/// Provider-compatible schema without the private `x-sensitive` extension.
/// # Returns
/// The operation result.
fn provider_schema(schema: &serde_json::Value) -> serde_json::Value {
    match schema {
        serde_json::Value::Object(object) => {
            let mut filtered = serde_json::Map::new();
            for (key, value) in object {
                if key != "x-sensitive" {
                    filtered.insert(key.clone(), provider_schema(value));
                }
            }
            serde_json::Value::Object(filtered)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(provider_schema).collect())
        }
        value => value.clone(),
    }
}
