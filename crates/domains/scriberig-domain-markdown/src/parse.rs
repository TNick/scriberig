//! Parses Markdown into the adapter's semantic tree.

#![allow(
    clippy::format_push_string,
    clippy::match_same_arms,
    clippy::too_many_lines,
    clippy::unnested_or_patterns
)]

use crate::mapping::{MarkdownBlock, MarkdownDocument};

/// Converts parser heading levels to stable numeric levels.
///
/// - `level`: Parser heading level.
///
/// Numeric heading depth from one through six.
/// # Returns
/// The operation result.
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
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Parses supported Markdown syntax into semantic blocks.
///
/// - `source`: Markdown source text.
///
/// A semantic Markdown document preserving block order.
#[must_use]
/// # Returns
/// The operation result.
pub fn parse(source: &str) -> MarkdownDocument {
    let parser = Parser::new_ext(source, Options::all());
    let mut blocks = Vec::new();
    let mut text = String::new();
    let mut list: Option<(bool, Vec<String>)> = None;
    let mut table: Option<Vec<Vec<String>>> = None;
    let mut row: Vec<String> = Vec::new();
    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                text.clear();
                text.push(char::from_digit(u32::from(heading_level(level)), 10).unwrap_or('1'));
                text.push('\0');
            }
            Event::End(TagEnd::Heading(_)) => {
                let value = text.split_off(0);
                let (level, body) = value.split_once('\0').unwrap_or(("1", value.as_str()));
                blocks.push(MarkdownBlock::Heading {
                    level: level.parse().unwrap_or(1),
                    text: body.trim().to_owned(),
                });
            }
            Event::Start(Tag::Paragraph) => text.clear(),
            Event::End(TagEnd::Paragraph) => {
                if !text.trim().is_empty() {
                    blocks.push(MarkdownBlock::Paragraph(text.trim().to_owned()));
                }
            }
            Event::Text(value) | Event::Code(value) => text.push_str(&value),
            Event::Start(Tag::List(start)) => list = Some((start.is_some(), Vec::new())),
            Event::Start(Tag::Item) => text.clear(),
            Event::End(TagEnd::Item) => {
                if let Some((_, items)) = &mut list {
                    items.push(text.trim().to_owned());
                }
            }
            Event::End(TagEnd::List(_)) => {
                if let Some((ordered, items)) = list.take() {
                    blocks.push(MarkdownBlock::List { ordered, items });
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                text.clear();
                if let CodeBlockKind::Fenced(language) = kind {
                    text.push_str(&format!("\u{0}{language}\n"));
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                let value = text.split_off(0);
                let language = value.strip_prefix('\0').and_then(|rest| {
                    rest.split_once('\n')
                        .map(|(name, body)| (name.to_owned(), body.to_owned()))
                });
                let (language, body) =
                    language.map_or((None, value), |(lang, body)| (Some(lang), body));
                blocks.push(MarkdownBlock::CodeBlock {
                    language,
                    text: body.trim_end().to_owned(),
                });
            }
            Event::Start(Tag::BlockQuote) => text.clear(),
            Event::End(TagEnd::BlockQuote) => {
                blocks.push(MarkdownBlock::Quote(text.trim().to_owned()));
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                text.clear();
                text.push_str(&format!("\u{1}{dest_url}\n"));
            }
            Event::End(TagEnd::Link) => {
                let value = text.split_off(0);
                if let Some((target, visible)) = value
                    .strip_prefix('\x01')
                    .and_then(|rest| rest.split_once('\n'))
                {
                    blocks.push(MarkdownBlock::Link {
                        text: visible.to_owned(),
                        target: target.to_owned(),
                    });
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                text.clear();
                text.push_str(&format!("\u{2}{dest_url}\n"));
            }
            Event::End(TagEnd::Image) => {
                let value = text.split_off(0);
                if let Some((target, alt)) = value
                    .strip_prefix('\x02')
                    .and_then(|rest| rest.split_once('\n'))
                {
                    blocks.push(MarkdownBlock::Image {
                        alt: alt.to_owned(),
                        target: target.to_owned(),
                    });
                }
            }
            Event::Start(Tag::Table(_)) => {
                table = Some(Vec::new());
                row.clear();
            }
            Event::End(TagEnd::Table) => {
                if let Some(rows) = table.take() {
                    blocks.push(MarkdownBlock::Table(rows));
                }
            }
            Event::Start(Tag::TableHead | Tag::TableRow) => row.clear(),
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                if let Some(rows) = &mut table {
                    rows.push(std::mem::take(&mut row));
                }
            }
            Event::Start(Tag::TableCell) => text.clear(),
            Event::End(TagEnd::TableCell) => row.push(text.trim().to_owned()),
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            _ => {}
        }
    }
    MarkdownDocument { blocks }
}
