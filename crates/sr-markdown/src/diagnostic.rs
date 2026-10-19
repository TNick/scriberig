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
    /// - `code`: Stable diagnostic code.
    /// - `message`: Human-readable explanation.
    ///
    /// A diagnostic without a source location.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            offset: None,
        }
    }
}
