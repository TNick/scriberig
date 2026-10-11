//! Renders semantic Markdown deterministically from an explicit style.

use crate::{
    model::{ListItem, MarkdownBlock, MarkdownDocument, Section},
    style::MarkdownStyle,
};

/// Renders a semantic document with the default style.
///
/// # Parameters
/// - `document`: Semantic document to render.
///
/// # Returns
/// Deterministic Markdown text.
#[must_use]
pub fn render(document: &MarkdownDocument) -> String {
    render_document(document, &MarkdownStyle::default())
}

/// Renders a semantic document with an exact style.
///
/// # Parameters
/// - `document`: Semantic document to render.
/// - `style`: Formatting choices to apply.
///
/// # Returns
/// Deterministic Markdown text.
#[must_use]
pub fn render_document(document: &MarkdownDocument, style: &MarkdownStyle) -> String {
    let style = style
        .clone()
        .validate()
        .unwrap_or_else(|_| MarkdownStyle::default());
    let mut output = String::new();
    let mut first = true;
    for block in &document.blocks {
        if !first {
            output.push_str(&"\n".repeat(style.blank_lines_between_blocks + 1));
        }
        first = false;
        render_block(block, &style, &mut output);
    }
    if style.final_newline {
        if !output.ends_with('\n') {
            output.push('\n');
        }
    } else {
        while output.ends_with('\n') {
            output.pop();
        }
    }
    output
}

/// Renders one block and its nested content.
///
/// # Parameters
/// - `block`: Block to render.
/// - `style`: Formatting choices to apply.
/// - `output`: Destination string.
fn render_block(block: &MarkdownBlock, style: &MarkdownStyle, output: &mut String) {
    match block {
        MarkdownBlock::Heading { level, text } => {
            output.push_str(&"#".repeat(usize::from(*level).clamp(1, 6)));
            output.push(' ');
            output.push_str(text.trim());
        }
        MarkdownBlock::Paragraph(text) => output.push_str(text.trim()),
        MarkdownBlock::List { ordered, items } => render_list(*ordered, items, style, output),
        MarkdownBlock::Table { headers, rows } => render_table(headers, rows, style, output),
        MarkdownBlock::CodeBlock { language, text } => {
            output.push_str(&style.code_fence_preference);
            if let Some(language) = language {
                output.push_str(language.trim());
            }
            output.push('\n');
            output.push_str(text.trim_end());
            output.push('\n');
            output.push_str(&style.code_fence_preference);
        }
        MarkdownBlock::QuoteBlock(blocks) => {
            for (index, nested) in blocks.iter().enumerate() {
                if index > 0 {
                    output.push('\n');
                }
                let mut rendered = String::new();
                render_block(nested, style, &mut rendered);
                for line in rendered.lines() {
                    output.push_str("> ");
                    output.push_str(line);
                    output.push('\n');
                }
                output.pop();
            }
        }
        MarkdownBlock::Link { label, target } => {
            output.push('[');
            output.push_str(label.trim());
            output.push_str("](");
            output.push_str(target.trim());
            output.push(')');
        }
        MarkdownBlock::Image { alt, target } => {
            output.push_str("![");
            output.push_str(alt.trim());
            output.push_str("](");
            output.push_str(target.trim());
            output.push(')');
        }
        MarkdownBlock::Section(section) => render_section(section, style, output),
    }
}

/// Renders a section heading and its child blocks.
///
/// # Parameters
/// - `section`: Section to render.
/// - `style`: Formatting choices to apply.
/// - `output`: Destination string.
fn render_section(section: &Section, style: &MarkdownStyle, output: &mut String) {
    output.push_str(&"#".repeat(usize::from(section.level).clamp(1, 6)));
    output.push(' ');
    output.push_str(section.heading.trim());
    for block in &section.blocks {
        output.push_str(&"\n".repeat(style.blank_lines_between_blocks + 1));
        render_block(block, style, output);
    }
}

/// Renders a list using explicit marker choices.
///
/// # Parameters
/// - `ordered`: Whether numeric markers are used.
/// - `items`: List items in source order.
/// - `style`: Formatting choices to apply.
/// - `output`: Destination string.
fn render_list(ordered: bool, items: &[ListItem], style: &MarkdownStyle, output: &mut String) {
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        if ordered {
            output.push_str(&(index + 1).to_string());
            output.push(style.ordered_list_delimiter);
        } else {
            output.push(style.list_marker);
        }
        output.push(' ');
        for (block_index, block) in item.blocks.iter().enumerate() {
            if block_index > 0 {
                output.push('\n');
                let mut nested = String::new();
                render_block(block, style, &mut nested);
                for line in nested.lines() {
                    output.push_str("  ");
                    output.push_str(line);
                    output.push('\n');
                }
                output.pop();
            } else {
                render_block(block, style, output);
            }
        }
    }
}

/// Renders a table with stable spacing and a delimiter row.
///
/// # Parameters
/// - `headers`: Header cells.
/// - `rows`: Body rows.
/// - `style`: Formatting choices to apply.
/// - `output`: Destination string.
fn render_table(
    headers: &[String],
    rows: &[Vec<String>],
    style: &MarkdownStyle,
    output: &mut String,
) {
    render_table_row(headers, style, output);
    output.push('\n');
    let separators = headers.iter().map(|_| "---").collect::<Vec<_>>();
    render_table_row(
        &separators
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
        style,
        output,
    );
    for row in rows {
        output.push('\n');
        render_table_row(row, style, output);
    }
}

/// Renders one table row.
///
/// # Parameters
/// - `cells`: Cells in source order.
/// - `style`: Formatting choices to apply.
/// - `output`: Destination string.
fn render_table_row(cells: &[String], style: &MarkdownStyle, output: &mut String) {
    let padding = " ".repeat(style.table_spacing);
    output.push('|');
    for cell in cells {
        output.push_str(&padding);
        output.push_str(cell.trim());
        output.push_str(&padding);
        output.push('|');
    }
}
