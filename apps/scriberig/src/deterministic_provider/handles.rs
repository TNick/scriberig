//! Extracts stable semantic handles from upstream tool results.

/// Parses ordered handles from a tool-result projection.
///
/// # Parameters
/// - `message`: Text projected from an upstream tool result.
///
/// # Returns
/// The parsed stable semantic handles.
///
/// # Errors
/// Returns an error when the result omits or corrupts its handle list.
pub(crate) fn parse_handles(message: &str) -> Result<Vec<String>, String> {
    let values = message
        .split_once("handles=[")
        .and_then(|(_, value)| value.split_once(']'))
        .map(|(values, _)| values)
        .ok_or_else(|| "tool result omitted handles".to_owned())?;
    serde_json::from_str(&format!("[{values}]"))
        .map_err(|error| format!("invalid tool result handles: {error}"))
}
