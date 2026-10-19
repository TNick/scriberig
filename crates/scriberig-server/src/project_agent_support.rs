//! Creates normalized responses for the deterministic project provider.

use scriberig_agent_runtime::provider::{FinishReason, ModelResponse, ProviderMetadata, ToolCall};

/// Creates a normalized provider response.
///
/// - `tool_call`: Optional tool call emitted by the fake provider.
/// - `text`: Optional final provider text.
/// - `super`: Parent module visibility.
///
/// A provider response with the corresponding finish reason.
/// # Returns
/// The operation result.
pub(super) fn response(tool_call: Option<ToolCall>, text: Option<&str>) -> ModelResponse {
    ModelResponse {
        text: text.map(str::to_owned),
        tool_calls: tool_call.into_iter().collect(),
        finish_reason: if text.is_some() {
            FinishReason::Stop
        } else {
            FinishReason::ToolCall
        },
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}
