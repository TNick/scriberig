//! Parses stable semantic handles from deterministic tool results.

use scriberig_agent_runtime::provider::ProviderError;

/// Extracts ordered handles from one tool-result message.
///
/// - `message`: Normalized tool-result message.
///
/// The parsed handles.
/// # Returns
/// The handles in their emitted order.
/// # Errors
/// Returns malformed-response when the result omits or corrupts handles.
pub(crate) fn parse_handles(message: &str) -> Result<Vec<String>, ProviderError> {
    let values = message
        .split_once("handles=[")
        .and_then(|(_, value)| value.split_once(']'))
        .map(|(values, _)| values)
        .ok_or_else(|| {
            ProviderError::MalformedResponse("tool result omitted handles".to_owned())
        })?;
    serde_json::from_str(&format!("[{values}]")).map_err(|error| {
        ProviderError::MalformedResponse(format!("invalid tool result handles: {error}"))
    })
}
