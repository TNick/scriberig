//! Verifies the generic Markdown projection adapter lifecycle.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, SnapshotId};
use sr_markdown::{
    CanonicalNode, MARKDOWN_KIND, MarkdownAdapter, MarkdownBlock, MarkdownDocument, MarkdownStyle,
    ProjectionAdapter,
};
use sr_projection::{
    ProjectionKind, ProjectionSpecRef, ReconciliationContext, ReconciliationOutcome,
    SemanticScopeRef, SemanticStateRef,
};

/// Creates the exact reconciliation context used by adapter tests.
fn context(current: SnapshotId) -> ReconciliationContext {
    ReconciliationContext {
        base: SemanticStateRef(SnapshotId::new([1; 32])),
        current: SemanticStateRef(current),
        scope: SemanticScopeRef {
            entity: EntityId::new([2; 16]),
            revision: None,
        },
        specification: ProjectionSpecRef(EntityId::new([3; 16])),
        metadata: BTreeMap::default(),
    }
}

#[test]
/// Verifies reconciliation preserves canonical identity in the generic API.
fn editable_reconciliation_preserves_identity() {
    let document = MarkdownDocument::new(vec![MarkdownBlock::Paragraph("before".to_owned())]);
    let node = CanonicalNode {
        id: EntityId::new([7; 16]),
        parent: None,
        block: document.blocks[0].clone(),
        position: 0,
    };
    let adapter = MarkdownAdapter::with_canonical_nodes(
        document,
        vec![node.clone()],
        MarkdownStyle::default(),
    )
    .expect("valid style");
    let kind = ProjectionKind::new(MARKDOWN_KIND).expect("kind");
    let candidate = adapter.parse(&kind, b"after\n").expect("candidate");
    let ReconciliationOutcome::MigrationProposal(proposal) =
        adapter.reconcile(candidate, &context(SnapshotId::new([1; 32])))
    else {
        panic!("expected identity-preserving migration proposal");
    };
    assert!(matches!(
        proposal.operations.as_slice(),
        [sr_change::Operation::ReplaceRevision { entity, .. }] if *entity == node.id
    ));
}

#[test]
/// Verifies unsafe single-base imports are rejected explicitly.
fn stale_or_missing_identity_context_is_rejected() {
    let adapter =
        MarkdownAdapter::with_default_style(MarkdownDocument::new(vec![MarkdownBlock::Paragraph(
            "before".to_owned(),
        )]));
    let kind = ProjectionKind::new(MARKDOWN_KIND).expect("kind");
    let candidate = adapter.parse(&kind, b"after\n").expect("candidate");
    assert!(matches!(
        adapter.reconcile(candidate.clone(), &context(SnapshotId::new([1; 32]))),
        ReconciliationOutcome::InvalidExternalState(_)
    ));
    assert!(matches!(
        adapter.reconcile(candidate, &context(SnapshotId::new([4; 32]))),
        ReconciliationOutcome::InvalidExternalState(_)
    ));
}

#[test]
/// Verifies disjoint canonical and external edits merge through the adapter.
fn concurrent_disjoint_edits_produce_one_migration() {
    let base_document = MarkdownDocument::new(vec![MarkdownBlock::Paragraph("base".to_owned())]);
    let current_document =
        MarkdownDocument::new(vec![MarkdownBlock::Paragraph("canonical".to_owned())]);
    let node = CanonicalNode {
        id: EntityId::new([8; 16]),
        parent: None,
        block: base_document.blocks[0].clone(),
        position: 0,
    };
    let adapter = MarkdownAdapter::with_canonical_nodes_and_base(
        current_document,
        vec![node.clone()],
        base_document,
        MarkdownStyle::default(),
    )
    .expect("valid style");
    let kind = ProjectionKind::new(MARKDOWN_KIND).expect("kind");
    let candidate = adapter
        .parse(&kind, b"base\n\nexternal\n")
        .expect("candidate");

    let outcome = adapter.reconcile(candidate, &context(SnapshotId::new([4; 32])));
    let ReconciliationOutcome::MigrationProposal(proposal) = outcome else {
        panic!("expected merged migration proposal");
    };
    assert!(proposal.operations.iter().any(|operation| matches!(
        operation,
        sr_change::Operation::ReplaceRevision { entity, .. } if *entity == node.id
    )));
    assert!(
        proposal
            .operations
            .iter()
            .any(|operation| matches!(operation, sr_change::Operation::CreateEntity { .. }))
    );
}

#[test]
/// Verifies competing edits remain structured adapter conflicts.
fn concurrent_competing_edits_produce_structured_conflict() {
    let base_document = MarkdownDocument::new(vec![MarkdownBlock::Paragraph("base".to_owned())]);
    let current_document =
        MarkdownDocument::new(vec![MarkdownBlock::Paragraph("canonical".to_owned())]);
    let node = CanonicalNode {
        id: EntityId::new([9; 16]),
        parent: None,
        block: base_document.blocks[0].clone(),
        position: 0,
    };
    let adapter = MarkdownAdapter::with_canonical_nodes_and_base(
        current_document,
        vec![node],
        base_document,
        MarkdownStyle::default(),
    )
    .expect("valid style");
    let kind = ProjectionKind::new(MARKDOWN_KIND).expect("kind");
    let candidate = adapter.parse(&kind, b"external\n").expect("candidate");

    let outcome = adapter.reconcile(candidate, &context(SnapshotId::new([5; 32])));
    assert!(matches!(
        outcome,
        ReconciliationOutcome::Conflict(conflicts) if !conflicts.is_empty()
    ));
}
