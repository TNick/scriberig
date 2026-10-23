//! Parses tool result payloads used by semantic discovery branches.

/// Checks whether one tool result is a semantic-find projection.
///
/// - `message`: Provider-normalized tool result text.
///
/// # Returns
/// `true` for a parsed semantic-find result.
pub(super) fn is_semantic_find(message: &str) -> bool {
    parse_tool_result(message).is_some_and(|value| {
        value.get("object_type").and_then(|value| value.as_str()) == Some("semantic_find")
    })
}

/// Parses a direct or quoted JSON tool result from provider context text.
///
/// - `message`: Provider-visible tool output.
///
/// # Returns
/// Parsed JSON when the output contains an encoded object.
pub fn parse_tool_result(message: &str) -> Option<serde_json::Value> {
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

/// Verifies raw and JSON-string-wrapped tool results advance semantic search.
#[cfg(test)]
#[test]
fn semantic_find_result_is_recognized_when_json_is_quoted() {
    let raw = r#"{"object_type":"semantic_find","summary":"semantic find in project"}"#;
    let quoted = serde_json::to_string(raw).expect("serialize quoted tool output");

    assert!(is_semantic_find(raw));
    assert!(is_semantic_find(&quoted));
}
