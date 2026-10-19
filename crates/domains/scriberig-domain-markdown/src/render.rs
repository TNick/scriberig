//! Renders the semantic Markdown tree deterministically.

use crate::mapping::{MarkdownBlock, MarkdownDocument};

/// Renders a semantic Markdown document using the default stable style.
///
/// - `document`: Semantic document to render.
///
/// Deterministic Markdown text.
/// # Returns
/// The operation result.
pub fn render(document: &MarkdownDocument) -> String {
    document
        .blocks
        .iter()
        .map(render_block)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Renders one semantic block.
///
/// - `block`: Semantic block to render.
///
/// Deterministic Markdown for the block.
/// # Returns
/// The operation result.
fn render_block(block: &MarkdownBlock) -> String {
    match block {
        MarkdownBlock::Heading { level, text } => format!("{} {text}", "#".repeat(*level as usize)),
        MarkdownBlock::Paragraph(text) => text.clone(),
        MarkdownBlock::List { ordered, items } => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                if *ordered {
                    format!("{}. {item}", index + 1)
                } else {
                    format!("- {item}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        MarkdownBlock::CodeBlock { language, text } => {
            format!("```{}\n{text}\n```", language.as_deref().unwrap_or(""))
        }
        MarkdownBlock::Quote(text) => text
            .lines()
            .map(|line| format!("> {line}"))
            .collect::<Vec<_>>()
            .join("\n"),
        MarkdownBlock::Table(rows) => rows
            .iter()
            .map(|row| format!("| {} |", row.join(" | ")))
            .collect::<Vec<_>>()
            .join("\n"),
        MarkdownBlock::Link { text, target } => format!("[{text}]({target})"),
        MarkdownBlock::Image { alt, target } => format!("![{alt}]({target})"),
    }
}
