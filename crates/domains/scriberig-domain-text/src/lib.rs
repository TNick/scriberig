//! Represents generic UTF-8 text as deterministic line entities.

use serde::{Deserialize, Serialize};

/// A UTF-8 text document with its complete source and ordered lines.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextDocument {
    /// Complete source text without decoding or normalization.
    pub source: String,
    /// Lines in source order, including a final empty line after a trailing newline.
    pub lines: Vec<TextLine>,
}

/// One line in a generic text document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TextLine {
    /// Zero-based line ordinal.
    pub index: usize,
    /// Line content without the line terminator.
    pub content: String,
}

/// Parses UTF-8 text into a deterministic line document.
///
/// - `source`: UTF-8 source text.
///
/// A document retaining the complete source and every line.
#[must_use]
/// # Returns
/// The operation result.
pub fn parse(source: &str) -> TextDocument {
    let lines = source
        .split('\n')
        .enumerate()
        .map(|(index, line)| TextLine {
            index,
            content: line.strip_suffix('\r').unwrap_or(line).to_owned(),
        })
        .collect();
    TextDocument {
        source: source.to_owned(),
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Preserves line order, CRLF normalization, and a trailing empty line.
    #[test]
    fn preserves_text_lines() {
        let document = parse("first\r\nsecond\n");
        assert_eq!(document.lines[0].content, "first");
        assert_eq!(document.lines[1].content, "second");
        assert_eq!(document.lines[2].content, "");
    }
}
