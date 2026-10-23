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
    if let Some(value) = result_object(message)
        && let Some(handles) = value.get("handles").and_then(serde_json::Value::as_array)
    {
        return Ok(handles
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect());
    }
    let values = message
        .split_once("handles=[")
        .and_then(|(_, value)| value.split_once(']'))
        .map(|(values, _)| values)
        .ok_or_else(|| "tool result omitted handles".to_owned())?;
    serde_json::from_str(&format!("[{values}]"))
        .map_err(|error| format!("invalid tool result handles: {error}"))
}

/// Decodes a direct or JSON-string-wrapped tool result object.
///
/// # Parameters
/// - `message`: Text projected from an upstream tool result.
///
/// # Returns
/// The result object when the text contains valid JSON.
fn result_object(message: &str) -> Option<serde_json::Value> {
    let mut candidate = message.trim().to_owned();
    for _ in 0..2 {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&candidate) {
            if let Some(inner) = value.as_str() {
                inner.trim().clone_into(&mut candidate);
                continue;
            }
            return Some(value);
        }
        candidate = candidate.get(candidate.find('{')?..)?.trim().to_owned();
    }
    serde_json::from_str(&candidate).ok()
}

#[cfg(test)]
mod tests {
    use super::parse_handles;

    /// Verifies handles survive provider context JSON-string wrapping.
    #[test]
    fn parses_handles_from_quoted_tool_result() {
        let raw = r#"{"handles":["candidate:cand_123"]}"#;
        let quoted = serde_json::to_string(raw).expect("serialize quoted tool result");

        assert_eq!(
            parse_handles(&quoted).expect("parse handles"),
            ["candidate:cand_123"]
        );
    }
}
