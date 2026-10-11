//! Parses supported Markdown into adapter-owned candidates.

#![allow(clippy::match_same_arms, clippy::too_many_lines)]

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::{
    candidate::CandidateDocument,
    diagnostic::MarkdownDiagnostic,
    model::{ListItem, MarkdownBlock, MarkdownDocument, Section},
};

/// Parses UTF-8 Markdown into a candidate tree.
///
/// # Errors
/// Returns a structured diagnostic when the input cannot be represented by the
/// supported semantic model.
///
/// # Parameters
/// - `source`: UTF-8 Markdown source text.
///
/// # Returns
/// A candidate document that retains source length for diagnostics.
pub fn parse(source: &str) -> Result<CandidateDocument, MarkdownDiagnostic> {
    let document = parse_document(source)?;
    Ok(CandidateDocument::new(document, source.len()))
}

/// Parses UTF-8 Markdown into a semantic document.
///
/// # Parameters
/// - `source`: UTF-8 Markdown source text.
///
/// # Returns
/// A normalized semantic Markdown document.
///
/// # Errors
/// Returns a diagnostic when parser state cannot be represented.
pub fn parse_document(source: &str) -> Result<MarkdownDocument, MarkdownDiagnostic> {
    let parser = Parser::new_ext(source, Options::all());
    let mut blocks = Vec::new();
    let mut text = String::new();
    let mut list: Option<(bool, Vec<ListItem>)> = None;
    let mut table: Option<Vec<Vec<String>>> = None;
    let mut row = Vec::new();
    let mut code_language = None;
    let mut in_quote = false;
    let mut quote_blocks = Vec::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { .. }) => text.clear(),
            Event::End(TagEnd::Heading(level)) => blocks.push(MarkdownBlock::Heading {
                level: heading_level(level),
                text: text.trim().to_owned(),
            }),
            Event::Start(Tag::Paragraph) => text.clear(),
            Event::End(TagEnd::Paragraph) => {
                if in_quote {
                    push_text_block(&mut quote_blocks, &text);
                } else {
                    push_text_block(&mut blocks, &text);
                }
            }
            Event::Text(value) | Event::Code(value) => text.push_str(&value),
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            Event::Start(Tag::List(start)) => list = Some((start.is_some(), Vec::new())),
            Event::Start(Tag::Item) => text.clear(),
            Event::End(TagEnd::Item) => {
                if let Some((_, items)) = &mut list {
                    items.push(ListItem::text(text.trim()));
                }
            }
            Event::End(TagEnd::List(_)) => {
                if let Some((ordered, items)) = list.take() {
                    blocks.push(MarkdownBlock::List { ordered, items });
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                text.clear();
                code_language = match kind {
                    CodeBlockKind::Fenced(value) if !value.is_empty() => Some(value.to_string()),
                    _ => None,
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                blocks.push(MarkdownBlock::CodeBlock {
                    language: code_language.take(),
                    text: text.trim_end().to_owned(),
                });
                text.clear();
            }
            Event::Start(Tag::BlockQuote) => {
                in_quote = true;
                text.clear();
                quote_blocks.clear();
            }
            Event::End(TagEnd::BlockQuote) => {
                in_quote = false;
                if !quote_blocks.is_empty() {
                    blocks.push(MarkdownBlock::QuoteBlock(std::mem::take(&mut quote_blocks)));
                }
                text.clear();
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                text.clear();
                text.push('\u{1}');
                text.push_str(&dest_url);
                text.push('\n');
            }
            Event::End(TagEnd::Link) => {
                if let Some((target, label)) = text
                    .strip_prefix('\u{1}')
                    .and_then(|value| value.split_once('\n'))
                {
                    blocks.push(MarkdownBlock::Link {
                        label: label.to_owned(),
                        target: target.to_owned(),
                    });
                }
                text.clear();
            }
            Event::Start(Tag::Table(_)) => {
                table = Some(Vec::new());
                row.clear();
            }
            Event::Start(Tag::TableHead | Tag::TableRow) => row.clear(),
            Event::Start(Tag::TableCell) => text.clear(),
            Event::End(TagEnd::TableCell) => row.push(text.trim().to_owned()),
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                if let Some(rows) = &mut table {
                    rows.push(std::mem::take(&mut row));
                }
            }
            Event::End(TagEnd::Table) => {
                let rows = table.take().ok_or_else(|| {
                    MarkdownDiagnostic::new("table-state", "table ended without rows")
                })?;
                let mut rows = rows.into_iter();
                let headers = rows.next().unwrap_or_default();
                blocks.push(MarkdownBlock::Table {
                    headers,
                    rows: rows.collect(),
                });
            }
            _ if in_quote => {}
            _ => {}
        }
    }
    Ok(group_sections(blocks))
}

/// Converts a parser heading stream into nested semantic sections.
///
/// # Parameters
/// - `blocks`: Flat parser blocks in source order.
///
/// # Returns
/// A document with heading hierarchy represented by sections.
fn group_sections(blocks: Vec<MarkdownBlock>) -> MarkdownDocument {
    let mut roots = Vec::new();
    let mut stack: Vec<Section> = Vec::new();
    for block in blocks {
        if let MarkdownBlock::Heading { level, text } = block {
            while stack.last().is_some_and(|section| section.level >= level) {
                finish_section(&mut roots, &mut stack);
            }
            stack.push(Section {
                level,
                heading: text,
                blocks: Vec::new(),
            });
        } else if let Some(section) = stack.last_mut() {
            section.blocks.push(block);
        } else {
            roots.push(block);
        }
    }
    while !stack.is_empty() {
        finish_section(&mut roots, &mut stack);
    }
    MarkdownDocument::new(roots)
}

/// Moves a completed section into its parent or the document root.
///
/// # Parameters
/// - `roots`: Document blocks receiving completed root sections.
/// - `stack`: Active section stack.
fn finish_section(roots: &mut Vec<MarkdownBlock>, stack: &mut Vec<Section>) {
    let section = stack.pop().expect("section stack is non-empty");
    let block = MarkdownBlock::Section(section);
    if let Some(parent) = stack.last_mut() {
        parent.blocks.push(block);
    } else {
        roots.push(block);
    }
}

/// Adds a non-empty paragraph to a block sequence.
///
/// # Parameters
/// - `blocks`: Destination block sequence.
/// - `text`: Candidate paragraph text.
fn push_text_block(blocks: &mut Vec<MarkdownBlock>, text: &str) {
    if !text.trim().is_empty() {
        blocks.push(MarkdownBlock::Paragraph(text.trim().to_owned()));
    }
}

/// Converts pulldown heading levels to a stable numeric depth.
///
/// # Parameters
/// - `level`: Parser heading level.
///
/// # Returns
/// Numeric heading depth from one through six.
fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
