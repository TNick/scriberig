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
/// Terminal controls become visible `\u{...}` text, Markdown inline markers
/// are escaped, and characters that only form structure at the start of a
/// line are escaped there. Identity text such as `ent_...` and `sna_...` is
/// preserved literally, because escaping its underscore would change the
/// exact identifier a user needs to copy or compare.
///
/// - `output`: Destination Markdown content.
/// - `value`: Untrusted semantic label, identifier, or preview.
pub(super) fn append_value(output: &mut String, value: &str) {
    let characters = value.chars().take(MAX_VALUE_CHARS).collect::<Vec<_>>();
    let mut line_start = true;
    let mut line_indent = 0_usize;
    let mut line_digits = 0_usize;
    let mut line_has_other = false;
    let mut previous: Option<char> = None;
    for (index, character) in characters.iter().copied().enumerate() {
        if character == '\n' {
            output.push('\n');
            line_start = true;
            line_indent = 0;
            line_digits = 0;
            line_has_other = false;
            previous = None;
            continue;
        }
        if character.is_control() {
            use std::fmt::Write as _;
            let _ = write!(output, "\\u{{{:x}}}", u32::from(character));
            line_start = false;
            previous = None;
            continue;
        }
        let next = characters.get(index + 1).copied();
        let inline_marker = matches!(character, '\\' | '`' | '*' | '<' | '>' | '~' | '|');
        let emphasis_at_word_boundary = character == '_'
            && !(previous.is_some_and(char::is_alphanumeric)
                && next.is_some_and(char::is_alphanumeric));
        let structural_at_line_start = matches!(character, '#' | '+' | '-' | '=') && line_start;
        let link_label = character == '[' && link_target_follows(&characters, index)
            || character == ']' && next == Some('(');
        let ordered_marker = character == '.'
            && line_digits > 0
            && !line_has_other
            && next.is_some_and(char::is_whitespace);
        if inline_marker
            || emphasis_at_word_boundary
            || structural_at_line_start
            || link_label
            || ordered_marker
        {
            output.push('\\');
        }
        output.push(character);
        if character == ' ' && line_start {
            line_indent += 1;
            line_start = line_indent <= 3;
        } else {
            line_start = false;
        }
        previous = Some(character);
        if character.is_ascii_digit() && !line_has_other {
            line_digits += 1;
        } else if character != ' ' || line_has_other {
            line_has_other = true;
        }
    }
    output.push_str("\n\n");
}

/// Reports whether a square-bracket label opens an inline Markdown link.
///
/// - `characters`: Bounded value characters being escaped.
/// - `index`: Index of the opening bracket.
///
/// # Returns
/// `true` when `](` follows on the same line, which is the only case where an
/// unescaped bracket pair becomes a link in the upstream renderer.
fn link_target_follows(characters: &[char], index: usize) -> bool {
    let line_end = characters[index..]
        .iter()
        .position(|character| *character == '\n')
        .map_or(characters.len(), |offset| index + offset);
    characters[index..line_end]
        .windows(2)
        .any(|window| window == [']', '('])
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

        assert!(
            !markdown
                .chars()
                .any(|character| character.is_control() && character != '\n'),
            "{markdown}"
        );
        assert!(markdown.contains("\\u{1b}"), "{markdown}");
        assert!(markdown.contains("\\u{7}"), "{markdown}");
        assert!(markdown.contains("\\*\\*"), "{markdown}");
    }

    /// Keeps exact identifier text copyable while still escaping structure.
    #[test]
    fn exact_identifiers_survive_markdown_escaping() {
        let mut markdown = String::new();
        append_value(
            &mut markdown,
            "Snapshot: sna_0123456789abcdef · entity ent_fedcba9876543210",
        );
        append_value(&mut markdown, "- list-like leading marker");
        append_value(&mut markdown, "start _emphasis_ and ***bold***");

        assert!(
            markdown.contains("sna_0123456789abcdef"),
            "snapshot identity must stay literal: {markdown}"
        );
        assert!(
            markdown.contains("ent_fedcba9876543210"),
            "entity identity must stay literal: {markdown}"
        );
        assert!(markdown.contains("\\- list-like"), "{markdown}");
        assert!(markdown.contains("\\_emphasis\\_"), "{markdown}");
        assert!(markdown.contains("\\*\\*\\*bold\\*\\*\\*"), "{markdown}");
        append_value(&mut markdown, "[linked](https://example.invalid/path)");
        append_value(&mut markdown, "1. ordered-list marker");
        append_value(&mut markdown, "> quote marker");
        append_value(&mut markdown, "Tool result hidden. [REDACTED]");
        assert!(
            markdown.contains("\\[linked\\](https://example.invalid/path)"),
            "{markdown}"
        );
        assert!(markdown.contains("1\\. ordered-list"), "{markdown}");
        assert!(markdown.contains("\\> quote marker"), "{markdown}");
        assert!(
            markdown.contains("Tool result hidden. [REDACTED]"),
            "ordinary punctuation and redaction labels must stay readable: {markdown}"
        );
    }
}
