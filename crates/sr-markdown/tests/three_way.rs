//! Verifies explicit base/current/external Markdown synchronization behavior.

use scriberig_core::SnapshotId;
use sr_markdown::{
    MarkdownConflictKind, MarkdownDocument, MarkdownStyle, ThreeWayOutcome, parse, synchronize,
    three_way_merge,
};

/// Parses a small Markdown semantic fixture.
///
/// - `source`: Markdown fixture text.
///
/// Parsed semantic document.
/// # Returns
/// The operation result.
fn document(source: &str) -> MarkdownDocument {
    parse(source).expect("Markdown fixture").document
}

/// Verifies disjoint canonical and external edits merge automatically.
#[test]
fn disjoint_changes_merge() {
    let base = document("one\n\ntwo\n");
    let current = document("one changed\n\ntwo\n");
    let external = document("one\n\ninserted\n\ntwo\n");
    let ThreeWayOutcome::Merged { document: merged } =
        three_way_merge(Some(&base), &current, &external)
    else {
        panic!("expected merged state");
    };
    assert_eq!(merged.blocks.len(), 3);
}

/// Verifies identical independent edits converge without conflict.
#[test]
fn same_result_converges() {
    let base = document("one\n");
    let result = document("same\n");
    assert!(matches!(
        three_way_merge(Some(&base), &result, &result),
        ThreeWayOutcome::Converged { .. }
    ));
}

/// Verifies incompatible edits retain all semantic sides.
#[test]
fn conflicting_text_is_structured() {
    let base = document("one\n");
    let current = document("current\n");
    let external = document("external\n");
    let ThreeWayOutcome::Conflict { conflicts } = three_way_merge(Some(&base), &current, &external)
    else {
        panic!("expected conflict");
    };
    assert_eq!(conflicts[0].kind, MarkdownConflictKind::SameBlockChanged);
    assert!(conflicts[0].base.is_some());
    assert!(conflicts[0].current.is_some());
    assert!(conflicts[0].external.is_some());
}

/// Verifies formatting-only external changes never produce external history.
#[test]
fn formatting_only_external_change_uses_current_canonical_state() {
    let base = document("- one\n");
    let current = document("- changed\n");
    let external = document("* one\n");
    assert!(matches!(
        three_way_merge(Some(&base), &current, &external),
        ThreeWayOutcome::FormattingOnly { document } if document == current
    ));
}

/// Verifies an unavailable base never falls back to a two-way merge.
#[test]
fn missing_base_is_explicit() {
    let current = document("current\n");
    let external = document("external\n");
    assert!(matches!(
        three_way_merge(None, &current, &external),
        ThreeWayOutcome::MissingSynchronizationBase
    ));
}

/// Verifies successful merge preparation renders bytes and defers base advancement.
#[test]
fn successful_merge_prepares_rerender_and_new_base() {
    let base = document("one\n");
    let current = document("current\n");
    let external = document("one\n\ninserted\n");
    let result = synchronize(
        Some(SnapshotId::new([1; 32])),
        Some(&base),
        SnapshotId::new([2; 32]),
        &current,
        &external,
        &MarkdownStyle::default(),
        SnapshotId::new([3; 32]),
    );
    assert!(result.rendered.is_some());
    assert_eq!(result.synchronization_base, Some(SnapshotId::new([3; 32])));
    assert!(!result.retryable);
}

/// Verifies conflicts retain the previous base and remain retryable.
#[test]
fn conflicted_merge_preserves_base_for_retry() {
    let base = document("one\n");
    let current = document("current\n");
    let external = document("external\n");
    let result = synchronize(
        Some(SnapshotId::new([1; 32])),
        Some(&base),
        SnapshotId::new([2; 32]),
        &current,
        &external,
        &MarkdownStyle::default(),
        SnapshotId::new([3; 32]),
    );
    assert!(result.rendered.is_none());
    assert_eq!(result.synchronization_base, Some(SnapshotId::new([1; 32])));
    assert!(result.retryable);
}

#[test]
/// Verifies an insertion does not conflict with a concurrent paragraph edit.
fn insertion_and_unrelated_edit_merge_by_base_anchor() {
    let base = document("one\n\ntwo\n");
    let current = document("one\n\ntwo changed\n");
    let external = document("inserted\n\none\n\ntwo\n");
    let ThreeWayOutcome::Merged { document: merged } =
        three_way_merge(Some(&base), &current, &external)
    else {
        panic!("expected base-anchored merge");
    };
    assert_eq!(merged, document("inserted\n\none\n\ntwo changed\n"));
}

#[test]
/// Verifies independent edits inside one section merge recursively.
fn nested_section_edits_merge_without_replacing_the_section() {
    let base = document("# Title\n\nfirst\n\nsecond\n");
    let current = document("# Title\n\nfirst changed\n\nsecond\n");
    let external = document("# Title\n\nfirst\n\ninserted\n\nsecond\n");
    let ThreeWayOutcome::Merged { document: merged } =
        three_way_merge(Some(&base), &current, &external)
    else {
        panic!("expected nested merge");
    };
    assert!(merged.blocks.iter().any(|block| matches!(
        block,
        sr_markdown::MarkdownBlock::Section(section)
            if section.blocks.len() == 3
    )));
}

#[test]
/// Verifies invalid rerender output is not promoted as a synchronization base.
fn invalid_rerender_is_structured_and_retryable() {
    let base = document("one\n");
    let current = MarkdownDocument::new(vec![sr_markdown::MarkdownBlock::Section(
        sr_markdown::Section {
            level: 7,
            heading: "invalid".to_owned(),
            blocks: Vec::new(),
        },
    )]);
    let result = synchronize(
        Some(SnapshotId::new([1; 32])),
        Some(&base),
        SnapshotId::new([2; 32]),
        &current,
        &base,
        &MarkdownStyle::default(),
        SnapshotId::new([3; 32]),
    );
    assert!(matches!(result.outcome, ThreeWayOutcome::Conflict { .. }));
    assert!(result.rendered.is_none());
    assert!(result.retryable);
    assert_eq!(result.synchronization_base, Some(SnapshotId::new([1; 32])));
}
