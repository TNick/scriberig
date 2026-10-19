//! Formats bounded semantic operation and model request details.

use std::fmt::Write;

/// Formats bounded redacted arguments as compact command options.
///
/// - `detail`: Structured presentation JSON retained for one invocation.
///
/// A safe, bounded command suffix or an empty string for invalid detail.
/// # Returns
/// The operation result.
pub(super) fn compact_tool_arguments(detail: &str) -> String {
    let Ok(serde_json::Value::Object(arguments)) = serde_json::from_str(detail) else {
        return String::new();
    };
    let mut suffix = String::new();
    for (name, value) in arguments.iter().take(8) {
        let value = value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned);
        let name = crate::inspector::sanitize(name)
            .chars()
            .take(64)
            .collect::<String>();
        let value = crate::inspector::sanitize(&value)
            .chars()
            .take(128)
            .collect::<String>();
        let _ = write!(suffix, " --{name} {value}");
    }
    suffix.chars().take(512).collect()
}

/// Formats grouped safe request sections for the opt-in prompt view.
///
/// - `detail`: Bounded captured provider-neutral request.
///
/// Sanitized model, message, tool-schema, and truncation sections.
/// # Returns
/// The operation result.
pub(super) fn format_model_request_detail(
    detail: &scriberig_protocol::ModelRequestDetailResponse,
) -> String {
    let mut sections = format!(
        "\nModel request\n  Model: {}\n  Thinking: {}",
        crate::inspector::sanitize(&detail.model),
        detail
            .thinking_level
            .as_deref()
            .map_or_else(|| "unspecified".to_owned(), crate::inspector::sanitize)
    );
    for message in &detail.messages {
        let _ = write!(
            sections,
            "\n  {}\n{}",
            crate::inspector::sanitize(&message.role),
            safe_multiline_text(&message.content)
                .lines()
                .map(|line| format!("    {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    if !detail.tools.is_empty() {
        sections.push_str("\n  Tools");
    }
    for tool in &detail.tools {
        let _ = write!(
            sections,
            "\n    {}: {}",
            crate::inspector::sanitize(&tool.name),
            crate::inspector::sanitize(&tool.description)
        );
        if let Some(parameters) = &tool.parameters {
            let _ = write!(sections, "\n{}", safe_multiline_text(parameters));
        }
    }
    if detail.truncated {
        sections.push_str("\n  Request capture truncated");
    }
    sections
}

/// Removes unsafe terminal controls while preserving safe line structure.
///
/// - `text`: Captured model-request text.
///
/// Sanitized request text with line breaks preserved.
/// # Returns
/// The operation result.
pub(super) fn safe_multiline_text(text: &str) -> String {
    text.chars()
        .filter(|character| *character == '\n' || !character.is_control())
        .collect()
}
