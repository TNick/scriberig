//! Verifies the generic Markdown projection adapter lifecycle.

use scriberig_core::{EntityId, SnapshotId};
use sr_markdown::{
    CanonicalNode, MARKDOWN_KIND, MarkdownAdapter, MarkdownBlock, MarkdownDocument, MarkdownStyle,
    ProjectionAdapter,
};
use sr_projection::{
    FidelityAssessment, ProjectionAdapterIdentity, ProjectionKind, ProjectionSpecRef,
    ReconciliationBaseline, ReconciliationContext, ReconciliationOutcome, RenderRequest,
    RenderedProjection, RendererVersion, RenderingContext, ResolvedStyle, SemanticScopeRef,
    SemanticStateRef, StyleValue,
};

/// Creates the exact reconciliation context used by adapter tests.
/// - `current`: Argument named `current`.
/// - `baseline_bytes`: Exact rendered bytes used as the reconciliation baseline.
///
/// # Returns
/// The operation result.
fn context(current: SnapshotId, baseline_bytes: &[u8]) -> ReconciliationContext {
    let base = SemanticStateRef(SnapshotId::new([1; 32]));
    let scope = SemanticScopeRef {
        entity: EntityId::from_bytes([2; 16]),
        revision: None,
    };
    let specification = ProjectionSpecRef(EntityId::from_bytes([3; 16]));
    let request = RenderRequest {
        specification,
        scope,
        base_state: base,
        kind: ProjectionKind::new("markdown").expect("kind"),
        style: ResolvedStyle::inline(StyleValue::new(Vec::new())),
        renderer: RendererVersion::new("markdown-test"),
        context: RenderingContext::default(),
    };
    let rendered = RenderedProjection::from_bytes(baseline_bytes.to_vec(), &request, None);
    ReconciliationContext {
        baseline: ReconciliationBaseline::new(
            rendered,
            ProjectionAdapterIdentity::new("markdown-test", "1").expect("adapter identity"),
            FidelityAssessment::lossless(),
        )
        .expect("valid baseline"),
        current: SemanticStateRef(current),
    }
}

#[test]
/// Verifies reconciliation preserves canonical identity in the generic API.
fn editable_reconciliation_preserves_identity() {
    let document = MarkdownDocument::new(vec![MarkdownBlock::Paragraph("before".to_owned())]);
    let node = CanonicalNode {
        id: EntityId::from_bytes([7; 16]),
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
        adapter.reconcile(candidate, &context(SnapshotId::new([1; 32]), b"before\n"))
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
        adapter.reconcile(
            candidate.clone(),
            &context(SnapshotId::new([1; 32]), b"before\n")
        ),
        ReconciliationOutcome::InvalidExternalState(_)
    ));
    assert!(matches!(
        adapter.reconcile(candidate, &context(SnapshotId::new([4; 32]), b"before\n")),
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
        id: EntityId::from_bytes([8; 16]),
        parent: None,
        block: base_document.blocks[0].clone(),
        position: 0,
    };
    let adapter = MarkdownAdapter::with_canonical_nodes(
        current_document,
        vec![node.clone()],
        MarkdownStyle::default(),
    )
    .expect("valid style");
    let kind = ProjectionKind::new(MARKDOWN_KIND).expect("kind");
    let candidate = adapter
        .parse(&kind, b"base\n\nexternal\n")
        .expect("candidate");

    let outcome = adapter.reconcile(candidate, &context(SnapshotId::new([4; 32]), b"base\n"));
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
        id: EntityId::from_bytes([9; 16]),
        parent: None,
        block: base_document.blocks[0].clone(),
        position: 0,
    };
    let adapter = MarkdownAdapter::with_canonical_nodes(
        current_document,
        vec![node],
        MarkdownStyle::default(),
    )
    .expect("valid style");
    let kind = ProjectionKind::new(MARKDOWN_KIND).expect("kind");
    let candidate = adapter.parse(&kind, b"external\n").expect("candidate");

    let outcome = adapter.reconcile(candidate, &context(SnapshotId::new([5; 32]), b"base\n"));
    assert!(matches!(
        outcome,
        ReconciliationOutcome::Conflict(conflicts) if !conflicts.is_empty()
    ));
}
