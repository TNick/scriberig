//! Defines complete safe message and tool details for model requests.

/// One safe complete message in an on-demand request detail record.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelRequestDetailMessage {
    /// Semantic message role.
    pub role: String,
    /// Provider-visible text content, excluding hidden reasoning and media.
    pub content: String,
    /// Safe tool name when this message is a tool result.
    #[serde(default)]
    pub tool_name: Option<String>,
    /// Provider tool-call ID associated with this tool result.
    #[serde(default)]
    pub tool_call_id: Option<String>,
    /// Safe tool calls from assistant context, excluding provider signatures.
    #[serde(default)]
    pub tool_calls: Vec<ModelRequestToolCall>,
    /// Ordered safe text and tool-call parts from the upstream message.
    #[serde(default)]
    pub parts: Vec<ModelRequestDetailPart>,
}

/// One ordered safe text or tool-call part from a model request message.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ModelRequestDetailPart {
    /// Provider-visible text content.
    Text(String),
    /// Safe normalized tool call without provider signatures.
    ToolCall(ModelRequestToolCall),
}

/// One provider-neutral assistant tool call retained for request inspection.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelRequestToolCall {
    /// Stable provider-facing tool name.
    pub name: String,
    /// Canonical JSON arguments supplied to the tool call.
    pub arguments: String,
    /// Provider call ID used to associate a later tool result, when present.
    #[serde(default)]
    pub id: Option<String>,
    /// Provider namespace for a dynamic or namespaced tool, when present.
    #[serde(default)]
    pub namespace: Option<String>,
}

/// One complete safe provider-facing tool definition in request detail.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelRequestDetailTool {
    /// Stable provider-facing tool name.
    pub name: String,
    /// Safe descriptive text.
    pub description: String,
    /// Canonical JSON Schema text.
    pub parameters: String,
    /// Upstream sampling constraint, when the tool defines one.
    pub constrained_sampling: Option<serde_json::Value>,
}
