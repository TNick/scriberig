//! Defines structured Markdown parsing and rendering diagnostics.

/// A diagnostic produced while handling external Markdown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownDiagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
    /// Optional source byte offset.
    pub offset: Option<usize>,
}

impl MarkdownDiagnostic {
    /// Creates a diagnostic with no source offset.
    ///
    /// # Parameters
    /// - `code`: Stable diagnostic code.
    /// - `message`: Human-readable explanation.
    ///
    /// # Returns
    /// A diagnostic without a source location.
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            offset: None,
        }
    }
}
