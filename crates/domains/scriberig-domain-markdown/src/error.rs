//! Defines explicit Markdown adapter failures.

/// Reports unsupported or malformed Markdown adapter input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkdownError {
    /// Input contains syntax not represented by the semantic tree.
    Unsupported(String),
}
