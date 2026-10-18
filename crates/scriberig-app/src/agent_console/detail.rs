//! Builds bounded, allowlisted detail for structured tool results.

/// Extracts safe presentation fields from a structured tool result.
///
/// # Parameters
/// - `bytes`: Structured result bytes from the trusted tool boundary.
///
/// # Returns
/// The allowlisted detail object and whether any field was truncated.
pub fn safe_tool_result_detail(bytes: &[u8]) -> (serde_json::Value, bool) {
    const MAX_TEXT_CHARS: usize = 4_096;
    let Some(object) = serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|value| value.as_object().cloned())
    else {
        return (serde_json::json!({}), true);
    };
    let mut detail = serde_json::Map::new();
    let mut truncated = false;
    let mut budget = MAX_TEXT_CHARS;
    for key in ["entity", "revision", "snapshot", "object_type", "summary"] {
        if let Some(serde_json::Value::String(text)) = object.get(key) {
            let value = text.chars().take(budget).collect::<String>();
            truncated |= value.chars().count() < text.chars().count();
            budget = budget.saturating_sub(value.chars().count());
            detail.insert(key.to_owned(), serde_json::Value::String(value));
        }
    }
    if let Some(serde_json::Value::Array(handles)) = object.get("handles") {
        let mut values = Vec::new();
        for handle in handles
            .iter()
            .filter_map(serde_json::Value::as_str)
            .take(100)
        {
            if budget == 0 {
                truncated = true;
                break;
            }
            let value = handle.chars().take(budget.min(256)).collect::<String>();
            truncated |= value.chars().count() < handle.chars().count();
            budget = budget.saturating_sub(value.chars().count());
            values.push(serde_json::Value::String(value));
        }
        if handles.len() > values.len() {
            truncated = true;
        }
        detail.insert("handles".to_owned(), serde_json::Value::Array(values));
    }
    (serde_json::Value::Object(detail), truncated)
}

#[cfg(test)]
mod tests {
    use super::safe_tool_result_detail;

    /// Verifies result details whitelist fields and enforce the text budget.
    #[test]
    fn result_detail_is_whitelisted_and_bounded() {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "summary": "x".repeat(100),
            "handles": ["handle-a"],
            "private_payload": "SECRET_MARKER",
        }))
        .expect("structured result");

        let (detail, truncated) = safe_tool_result_detail(&bytes);

        assert!(!truncated);
        assert_eq!(detail["summary"].as_str().unwrap().chars().count(), 100);
        assert_eq!(detail["handles"][0], "handle-a");
        assert!(!detail.to_string().contains("SECRET_MARKER"));

        let oversized = serde_json::to_vec(&serde_json::json!({
            "summary": "x".repeat(5_000),
        }))
        .expect("oversized structured result");
        let (detail, truncated) = safe_tool_result_detail(&oversized);
        assert!(truncated);
        assert_eq!(detail["summary"].as_str().unwrap().chars().count(), 4_096);
    }
}
