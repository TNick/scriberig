//! Defines OpenAI-compatible response wire structures.

use serde::Deserialize;

/// Deserializes one compatible provider response.
#[derive(Deserialize)]
pub(super) struct WireResponse {
    pub(super) choices: Vec<WireChoice>,
    pub(super) usage: Option<WireUsage>,
    pub(super) model: Option<String>,
}

#[derive(Deserialize)]
/// Deserializes one response choice.
pub(super) struct WireChoice {
    pub(super) message: WireAssistantMessage,
    pub(super) finish_reason: Option<String>,
}

#[derive(Deserialize)]
/// Deserializes the assistant message in a response choice.
pub(super) struct WireAssistantMessage {
    pub(super) content: Option<String>,
    pub(super) tool_calls: Option<Vec<WireToolCall>>,
}

#[derive(Deserialize)]
/// Deserializes one provider function call.
pub(super) struct WireToolCall {
    pub(super) id: String,
    pub(super) function: WireFunctionCall,
}

#[derive(Deserialize)]
/// Deserializes function-call name and JSON arguments.
pub(super) struct WireFunctionCall {
    pub(super) name: String,
    pub(super) arguments: String,
}

#[derive(Deserialize)]
/// Deserializes provider token usage metadata.
pub(super) struct WireUsage {
    #[serde(rename = "prompt_tokens")]
    pub(super) input: u64,
    #[serde(rename = "completion_tokens")]
    pub(super) output: u64,
    #[serde(rename = "total_tokens")]
    pub(super) total: u64,
}
