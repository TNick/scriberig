//! Defines the parser-independent semantic Markdown tree.

/// Represents one semantic Markdown document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownDocument {
    /// Ordered top-level blocks.
    pub blocks: Vec<MarkdownBlock>,
}

/// Represents one Markdown semantic block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkdownBlock {
    /// A heading with level and text.
    Heading {
        /// Heading nesting level.
        level: u8,
        /// Heading text.
        text: String,
    },
    /// A paragraph with semantic text.
    Paragraph(String),
    /// An ordered or unordered list.
    List {
        /// Whether numbering is ordered.
        ordered: bool,
        /// Ordered item text.
        items: Vec<String>,
    },
    /// A code block with optional language.
    CodeBlock {
        /// Optional fenced language.
        language: Option<String>,
        /// Code content.
        text: String,
    },
    /// A quote block.
    Quote(String),
    /// A table with ordered rows and cells.
    Table(Vec<Vec<String>>),
    /// A link with visible text and target.
    Link {
        /// Visible link text.
        text: String,
        /// Link destination.
        target: String,
    },
    /// An image reference with alt text and target.
    Image {
        /// Alternative image text.
        alt: String,
        /// Image destination.
        target: String,
    },
}
