//! Verifies deterministic Markdown parsing, rendering, and equivalence.

use sr_markdown::{
    MarkdownBlock, MarkdownStyle, equivalent, parse, render_document, semantically_equivalent,
};

#[test]
/// Verifies supported block kinds survive a parse and render round trip.
fn round_trip_preserves_supported_blocks() {
    let source = "# Architecture\n\nStorage is immutable.\n\n- one\n- two\n\n> quoted\n\n[docs](https://example.test)\n\n```rust\nlet x = 1;\n```\n\n| key | value |\n| --- | --- |\n| a | b |\n";
    let candidate = parse(source).expect("supported Markdown should parse");
    assert!(candidate.document.blocks.iter().any(|block| matches!(
        block,
        MarkdownBlock::Section(_) | MarkdownBlock::Heading { .. }
    )));
    assert!(contains_quote(&candidate.document.blocks));
    let rendered = render_document(&candidate.document, &MarkdownStyle::default());
    let reparsed = parse(&rendered).expect("rendered Markdown should parse");
    assert!(semantically_equivalent(&candidate, &reparsed));
}

/// Reports whether a block tree contains a quote block.
///
/// # Parameters
/// - `blocks`: Blocks to inspect recursively.
///
/// # Returns
/// `true` when a quote block is present.
fn contains_quote(blocks: &[MarkdownBlock]) -> bool {
    blocks.iter().any(|block| match block {
        MarkdownBlock::QuoteBlock(_) => true,
        MarkdownBlock::Section(section) => contains_quote(&section.blocks),
        _ => false,
    })
}

#[test]
/// Verifies supported list marker changes are formatting-only.
fn list_marker_and_spacing_changes_are_semantically_equivalent() {
    let left = parse("- one\n- two\n").expect("left input");
    let right = parse("* one\n* two\n").expect("right input");
    assert!(semantically_equivalent(&left, &right));
}

#[test]
/// Verifies changed text and heading depth alter semantic equality.
fn semantic_text_and_heading_changes_are_not_equivalent() {
    let original = parse("# Title\n\nOriginal text.\n").expect("original input");
    let changed_text = parse("# Title\n\nChanged text.\n").expect("text input");
    let changed_heading = parse("## Title\n\nOriginal text.\n").expect("heading input");
    assert!(!equivalent(&original.document, &changed_text.document));
    assert!(!equivalent(&original.document, &changed_heading.document));
}

#[test]
/// Verifies style changes alter bytes while preserving parsed semantics.
fn styles_change_bytes_without_changing_semantics() {
    let document = parse("- one\n- two\n").expect("input").document;
    let default_output = render_document(&document, &MarkdownStyle::default());
    let alternate = MarkdownStyle {
        list_marker: '*',
        final_newline: false,
        ..MarkdownStyle::default()
    };
    let alternate_output = render_document(&document, &alternate);
    assert_ne!(default_output, alternate_output);
    assert!(semantically_equivalent(
        &parse(&default_output).expect("default output"),
        &parse(&alternate_output).expect("alternate output")
    ));
}
