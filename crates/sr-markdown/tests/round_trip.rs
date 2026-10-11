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

#[test]
/// Verifies inline links and images survive semantic round trips.
fn inline_links_and_images_are_preserved() {
    let source = "A [link](https://example.test) and ![logo](logo.png) with `code`.\n";
    let candidate = parse(source).expect("inline Markdown should parse");
    let rendered = render_document(&candidate.document, &MarkdownStyle::default());
    let reparsed = parse(&rendered).expect("rendered Markdown should parse");
    assert!(semantically_equivalent(&candidate, &reparsed));
}

#[test]
/// Verifies unsupported Markdown is rejected instead of silently discarded.
fn unsupported_markdown_returns_a_diagnostic() {
    let error = parse("---\n").expect_err("thematic breaks are unsupported");
    assert_eq!(error.code, "unsupported-markdown");
}

#[test]
/// Verifies candidate paths are local diagnostics metadata, not entity IDs.
fn candidates_expose_structural_paths() {
    let candidate = parse("# Title\n\ntext\n").expect("candidate");
    assert_eq!(candidate.node_paths, vec![vec![0], vec![0, 0]]);
}

#[test]
/// Verifies candidates retain source-coordinate diagnostics metadata.
fn candidates_expose_source_spans() {
    let source = "# Title\n\ntext\n";
    let candidate = parse(source).expect("candidate");
    assert_ne!(candidate.source_spans.len(), 0);
    assert!(
        candidate
            .source_spans
            .iter()
            .all(|span| span.start < span.end && span.end <= source.len())
    );
}

#[test]
/// Verifies nested list items retain their parent/child structure.
fn nested_lists_round_trip_without_losing_the_outer_item() {
    let source = "- parent\n  - child\n";
    let candidate = parse(source).expect("nested list should parse");
    let rendered = render_document(&candidate.document, &MarkdownStyle::default());
    let reparsed = parse(&rendered).expect("rendered nested list should parse");
    assert!(semantically_equivalent(&candidate, &reparsed));
    assert!(
        matches!(
            &candidate.document.blocks[0],
            MarkdownBlock::List { items, .. }
                if items[0].blocks.iter().any(|block| matches!(block, MarkdownBlock::List { .. }))
        ),
        "candidate={candidate:?}"
    );
    assert_eq!(
        candidate.node_paths,
        vec![
            vec![0],
            vec![0, 0],
            vec![0, 0, 0],
            vec![0, 0, 1],
            vec![0, 0, 1, 0],
            vec![0, 0, 1, 0, 0],
        ]
    );
}
