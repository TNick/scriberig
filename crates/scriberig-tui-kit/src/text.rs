//! Defines safe external text and a width-aware text component adapter.

use std::fmt::Write as _;

use rpi_tui::Component;

/// Holds external text after terminal control characters are made visible.
///
/// # Public members
/// - [`ExternalText::new`]: Sanitizes text at the boundary.
/// - [`ExternalText::as_str`]: Borrows the safe plain text.
///
/// # Private members
/// - `value`: Stores the sanitized text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalText {
    value: String,
}

impl ExternalText {
    /// Creates safe display text from untrusted input.
    ///
    /// Newlines are retained as structure; other control characters become
    /// visible `\\u{...}` sequences before they reach a renderer.
    ///
    /// # Parameters
    /// - `value`: Untrusted text to sanitize.
    ///
    /// # Returns
    /// Safe plain text suitable for the normal rendering path.
    #[must_use]
    pub fn new(value: &str) -> Self {
        let mut safe = String::with_capacity(value.len());
        for character in value.chars() {
            if character == '\n' {
                safe.push(character);
            } else if character.is_control() {
                let _ = write!(safe, "\\u{{{:x}}}", u32::from(character));
            } else {
                safe.push(character);
            }
        }
        Self { value: safe }
    }

    /// Borrows the sanitized plain text.
    ///
    /// # Returns
    /// The safe text without terminal styling sequences.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

/// Wraps the upstream text component without exposing its type.
///
/// # Public members
/// - [`TextBlock::new`]: Creates a width-aware text block.
/// - [`TextBlock::render`]: Renders safe plain lines.
///
/// # Private members
/// - `component`: Holds the upstream rendering component.
pub struct TextBlock {
    component: rpi_tui::Text,
}

impl TextBlock {
    /// Creates a text block from external text.
    ///
    /// # Parameters
    /// - `text`: Text sanitized before upstream rendering.
    ///
    /// # Returns
    /// A wrapped text component.
    #[must_use]
    pub fn new(text: &ExternalText) -> Self {
        Self {
            component: rpi_tui::Text::new(text.as_str(), 0, 0),
        }
    }

    /// Renders the block and removes upstream styling from its output.
    ///
    /// # Parameters
    /// - `width`: Maximum rendering width in terminal columns.
    ///
    /// # Returns
    /// Safe plain text lines constrained to the requested width.
    #[must_use]
    pub fn render(&self, width: usize) -> Vec<ExternalText> {
        self.component
            .render(width)
            .iter()
            .map(|line| ExternalText::new(&rpi_tui::strip_ansi(line)))
            .collect()
    }
}

/// Measures terminal columns for sanitized text.
///
/// # Parameters
/// - `text`: Safe plain text whose display width is needed.
///
/// # Returns
/// The number of terminal columns occupied by the text.
#[must_use]
pub fn display_width(text: &ExternalText) -> usize {
    unicode_width::UnicodeWidthStr::width(text.as_str())
}
