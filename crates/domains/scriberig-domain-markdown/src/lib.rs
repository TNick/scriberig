//! Adapts Markdown documents to the universal semantic and projection architecture.

pub mod adapter;
pub mod classification;
pub mod error;
pub mod mapping;
pub mod parse;
pub mod render;
pub mod schema;
pub mod types;

pub use classification::{InformationClass, classify};
pub use mapping::{MarkdownBlock, MarkdownDocument};
pub use parse::parse;
pub use render::render;
pub use schema::{OrderedRelation, markdown_schema};

#[cfg(test)]
mod tests {
    use super::{MarkdownBlock, parse, render};

    /// Verifies supported Markdown preserves semantic block order.
    #[test]
    fn parse_and_render_are_deterministic() {
        let source = "# Title\n\nParagraph.\n\n- one\n- two\n\n```rust\nlet x = 1;\n```";
        let document = parse(source);
        assert!(matches!(
            document.blocks[0],
            MarkdownBlock::Heading { level: 1, .. }
        ));
        assert!(matches!(document.blocks[1], MarkdownBlock::Paragraph(_)));
        assert_eq!(render(&document), render(&document));
        assert!(render(&document).contains("```rust"));
    }
}
