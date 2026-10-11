//! Defines the adapter-owned semantic Markdown model.

/// A parsed or canonical Markdown document in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownDocument {
    /// Top-level blocks in document order.
    pub blocks: Vec<MarkdownBlock>,
}

impl MarkdownDocument {
    /// Creates a document from ordered blocks.
    ///
    /// # Parameters
    /// - `blocks`: Top-level blocks in source order.
    ///
    /// # Returns
    /// A semantic Markdown document.
    #[must_use]
    pub fn new(blocks: Vec<MarkdownBlock>) -> Self {
        Self { blocks }
    }
}

/// One semantic Markdown block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkdownBlock {
    /// A heading with its semantic depth and inline text.
    Heading {
        /// Heading depth from one through six.
        level: u8,
        /// Heading inline text.
        text: String,
    },
    /// A paragraph with normalized inline source text.
    Paragraph(String),
    /// An ordered or unordered list.
    List {
        /// Whether the list uses numeric markers.
        ordered: bool,
        /// Items in source order.
        items: Vec<ListItem>,
    },
    /// A table with a header row and body rows.
    Table {
        /// Header cells in source order.
        headers: Vec<String>,
        /// Body rows in source order.
        rows: Vec<Vec<String>>,
    },
    /// A fenced or indented code block.
    CodeBlock {
        /// Optional fenced language tag.
        language: Option<String>,
        /// Exact code content without fence markers.
        text: String,
    },
    /// A block quote represented by its nested semantic blocks.
    QuoteBlock(Vec<MarkdownBlock>),
    /// A standalone link block.
    Link {
        /// Visible link label.
        label: String,
        /// Link destination.
        target: String,
    },
    /// An image reference with alternative text and destination.
    Image {
        /// Alternative text.
        alt: String,
        /// Image destination.
        target: String,
    },
    /// A section grouping a heading and all blocks until the next heading.
    Section(Section),
}

/// One ordered section in a Markdown document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Section {
    /// Heading depth from one through six.
    pub level: u8,
    /// Heading text.
    pub heading: String,
    /// Blocks belonging to this section.
    pub blocks: Vec<MarkdownBlock>,
}

/// One list item and its nested blocks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListItem {
    /// Item blocks in source order.
    pub blocks: Vec<MarkdownBlock>,
}

impl ListItem {
    /// Creates a list item from its blocks.
    ///
    /// # Parameters
    /// - `blocks`: Item blocks in source order.
    ///
    /// # Returns
    /// A list item.
    #[must_use]
    pub fn new(blocks: Vec<MarkdownBlock>) -> Self {
        Self { blocks }
    }

    /// Creates a simple text list item.
    ///
    /// # Parameters
    /// - `text`: Text for the first paragraph.
    ///
    /// # Returns
    /// A list item containing one paragraph.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(vec![MarkdownBlock::Paragraph(text.into())])
    }
}
