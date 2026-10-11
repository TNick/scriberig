//! Verifies deterministic Markdown identity matching and proposals.

use scriberig_core::{EntityId, SnapshotId};
use sr_change::Operation;
use sr_markdown::{
    CanonicalNode, MarkdownBlock, MarkdownReconciliation, NodeMatch, match_nodes, parse, reconcile,
};

/// Creates a canonical paragraph node with a stable identity.
///
/// # Parameters
/// - `id`: Byte repeated to construct the entity identity.
/// - `position`: Semantic position in the parent collection.
/// - `text`: Paragraph content.
///
/// # Returns
/// A canonical paragraph node.
fn node(id: u8, position: usize, text: &str) -> CanonicalNode {
    CanonicalNode {
        id: EntityId::new([id; 16]),
        parent: None,
        block: MarkdownBlock::Paragraph(text.to_owned()),
        position,
    }
}

/// Verifies a changed paragraph retains its entity identity.
#[test]
fn changed_paragraph_produces_revision_replacement() {
    let base = vec![node(1, 0, "before")];
    let candidate = parse("before changed\n").expect("candidate");
    let result = reconcile(
        SnapshotId::new([9; 32]),
        &base,
        &candidate,
        "architecture.md",
    );
    let MarkdownReconciliation::MigrationProposal(proposal) = result else {
        panic!("expected migration proposal");
    };
    assert!(matches!(
        proposal.operations.as_slice(),
        [Operation::ReplaceRevision { entity, .. }] if *entity == base[0].id
    ));
}

/// Verifies unchanged nodes are matched and inserted nodes are created.
#[test]
fn insertion_preserves_existing_match() {
    let base = vec![node(1, 0, "one"), node(2, 1, "three")];
    let candidate = parse("one\n\ntwo\n\nthree\n").expect("candidate");
    let report = match_nodes(&base, &candidate.document);
    assert!(report.matches.iter().any(|item| matches!(
        item,
        NodeMatch::Matched { existing, .. } if *existing == base[0].id
    )));
    assert!(
        report
            .matches
            .iter()
            .any(|item| matches!(item, NodeMatch::New { .. }))
    );
}

/// Verifies formatting-only input produces no semantic migration.
#[test]
fn formatting_only_change_is_a_no_op() {
    let base = vec![CanonicalNode {
        id: EntityId::new([1; 16]),
        parent: None,
        block: MarkdownBlock::List {
            ordered: false,
            items: vec![sr_markdown::model::ListItem::text("one")],
        },
        position: 0,
    }];
    let candidate = parse("* one\n").expect("candidate");
    assert!(matches!(
        reconcile(SnapshotId::new([9; 32]), &base, &candidate, "file.md"),
        MarkdownReconciliation::NoSemanticChange
    ));
}

/// Verifies duplicate semantic content remains explicitly ambiguous.
#[test]
fn duplicate_content_is_not_assigned_by_position() {
    let base = vec![node(1, 0, "same"), node(2, 1, "same")];
    let candidate = parse("same\n").expect("candidate");
    let report = match_nodes(&base, &candidate.document);
    assert!(
        matches!(report.matches.as_slice(), [NodeMatch::Ambiguous { possible, .. }] if possible.len() == 2)
    );
}

/// Verifies a reordered child retains identity and emits a collection move.
#[test]
fn reorder_emits_collection_move_for_nested_nodes() {
    let collection = EntityId::new([8; 16]);
    let mut first = node(1, 0, "one");
    first.parent = Some(collection);
    let mut second = node(2, 1, "two");
    second.parent = Some(collection);
    let base = vec![first, second];
    let candidate = parse("two\n\none\n").expect("candidate");
    let result = reconcile(SnapshotId::new([9; 32]), &base, &candidate, "file.md");
    let MarkdownReconciliation::MigrationProposal(proposal) = result else {
        panic!("expected move proposal");
    };
    assert!(proposal.operations.iter().any(|operation| matches!(
        operation,
        Operation::MoveCollectionMember { collection: actual, member, .. }
            if *actual == collection && *member == base[1].id
    )));
}
