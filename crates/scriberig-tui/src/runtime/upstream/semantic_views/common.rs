//! Shares bounded formatting helpers for upstream-rendered semantic views.

use crate::inspector::LoadState;

/// Maximum rows rendered from one bounded `ScribeRig` view model.
pub(super) const MAX_ROWS: usize = 200;

/// Maximum characters rendered from one untrusted semantic value.
const MAX_VALUE_CHARS: usize = 800;

/// Renders one effect-backed model and a safe loading or failure notice.
///
/// - `title`: Semantic view name.
/// - `state`: Request state for this view.
/// - `render`: Projection-specific formatter for a ready model.
///
/// # Returns
/// The title, request status, and ready model content.
pub(super) fn render_load_state<T>(
    title: &str,
    state: &LoadState<T>,
    render: impl FnOnce(&T, &mut String),
) -> String {
    let mut output = format!("# {title}\n\n");
    match state {
        LoadState::NotRequested => append_value(&mut output, "No request has been made."),
        LoadState::Loading { .. } => append_value(&mut output, "Loading…"),
        LoadState::Ready(model) => render(model, &mut output),
        LoadState::Failed { message, stale, .. } => {
            append_value(&mut output, message);
            if *stale {
                append_value(&mut output, "Showing the last successful result.");
            }
        }
    }
    output
}

/// Renders a load-state notice when no retained model is available.
///
/// - `output`: Existing view heading and context.
/// - `title`: Semantic source of the request state.
/// - `state`: Request state to describe.
///
/// # Returns
/// The content with a safe loading, empty, or failure notice.
pub(super) fn append_load_notice<T>(
    mut output: String,
    title: &str,
    state: &LoadState<T>,
) -> String {
    match state {
        LoadState::NotRequested => append_value(&mut output, &format!("{title} is not loaded.")),
        LoadState::Loading { .. } => append_value(&mut output, &format!("Loading {title}…")),
        LoadState::Ready(_) => append_value(&mut output, "No review data is available."),
        LoadState::Failed { message, stale, .. } => {
            append_value(&mut output, message);
            if *stale {
                append_value(&mut output, "Showing the last successful result.");
            }
        }
    }
    output
}

/// Appends a bounded semantic value as escaped Markdown text.
///
/// - `output`: Destination Markdown content.
/// - `value`: Untrusted semantic label, identifier, or preview.
pub(super) fn append_value(output: &mut String, value: &str) {
    let clean = crate::inspector::sanitize(value);
    for character in clean.chars().take(MAX_VALUE_CHARS) {
        if matches!(
            character,
            '\\' | '`'
                | '*'
                | '_'
                | '{'
                | '}'
                | '['
                | ']'
                | '<'
                | '>'
                | '#'
                | '+'
                | '-'
                | '.'
                | '!'
                | '|'
        ) {
            output.push('\\');
        }
        output.push(character);
    }
    output.push_str("\n\n");
}

/// Appends a bounded collection of semantic values under a heading.
///
/// - `output`: Destination Markdown content.
/// - `heading`: Safe category label.
/// - `values`: Semantic identifiers to display.
pub(super) fn append_values(output: &mut String, heading: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    append_value(output, heading);
    for value in values.iter().take(MAX_ROWS) {
        append_value(output, value);
    }
}

/// Formats an optional typed identity without inventing a current value.
///
/// - `value`: Exact immutable identity when present.
///
/// # Returns
/// The identity string or `none`.
pub(super) fn optional_id<T: std::fmt::Display>(value: Option<T>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::append_value;

    /// Keeps the shared hostile terminal corpus as inert Markdown data.
    #[test]
    fn hostile_corpus_values_are_sanitized_and_markdown_escaped() {
        let corpus: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../../../tests/fixtures/terminal_hostile_corpus.json"
        ))
        .expect("hostile corpus JSON");
        let mut markdown = String::new();
        for entry in corpus {
            append_value(
                &mut markdown,
                entry["value"].as_str().expect("hostile corpus value"),
            );
        }

        assert!(!markdown.chars().any(char::is_control), "{markdown}");
        assert!(markdown.contains("\\u{1b}"), "{markdown}");
        assert!(markdown.contains("\\u{7}"), "{markdown}");
        assert!(markdown.contains("\\*\\*"), "{markdown}");
    }
}
