//! Verifies reconciliation baselines bind exact output bytes to render inputs.

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    FidelityAssessment, ProjectionAdapterIdentity, ProjectionEvidence, ProjectionKind,
    ProjectionRef, ProjectionSpecRef, ReconciliationBaseline, RenderRequest, RendererVersion,
    RenderingContext, ResolvedStyle, SemanticScopeRef, SemanticStateRef, StyleValue,
};

/// Builds a deterministic source render request for baseline tests.
///
/// - `snapshot`: Exact semantic snapshot represented by the output.
///
/// A request with exact scope, style, renderer, and specification.
///
/// # Returns
/// The deterministic request used by baseline tests.
fn request(snapshot: SnapshotId) -> RenderRequest {
    RenderRequest {
        specification: ProjectionSpecRef(EntityId::from_bytes([2; 16])),
        scope: SemanticScopeRef {
            entity: EntityId::from_bytes([3; 16]),
            revision: None,
        },
        base_state: SemanticStateRef(snapshot),
        kind: ProjectionKind::new("source").expect("kind"),
        style: ResolvedStyle::inline(StyleValue::new(b"spaces=2".to_vec())),
        renderer: RendererVersion::new("renderer-4"),
        context: RenderingContext::default(),
    }
}

/// Builds the adapter identity shared by the baseline cases.
///
/// The exact adapter identity.
///
/// # Returns
/// The deterministic adapter identity used by baseline tests.
fn adapter() -> ProjectionAdapterIdentity {
    ProjectionAdapterIdentity::new("source-adapter", "3").expect("valid adapter identity")
}

/// Rejects a rendered output whose recorded digest does not match its bytes.
#[test]
fn constructor_rejects_a_mismatched_output_digest() {
    let request = request(SnapshotId::new([1; 32]));
    let mut rendered =
        sr_projection::RenderedProjection::from_bytes(b"exact baseline".to_vec(), &request, None);
    rendered.output_hash.as_mut().expect("hash exists").digest[0] ^= 0xff;

    assert_eq!(
        ReconciliationBaseline::new(rendered, adapter(), FidelityAssessment::lossless())
            .expect_err("a changed digest cannot authenticate other bytes"),
        "rendered baseline digest does not match exact output bytes"
    );
}

/// Retains exact baseline bytes across canonical advancement and rerendering.
#[test]
fn output_update_keeps_prior_style_and_records_new_state_and_bytes() {
    let original_state = SemanticStateRef(SnapshotId::new([1; 32]));
    let original = ReconciliationBaseline::from_render(
        &request(original_state.0),
        Some(ProjectionRef(EntityId::from_bytes([4; 16]))),
        b"first rendering".to_vec(),
        adapter(),
        FidelityAssessment::lossless(),
    );
    let next_state = SemanticStateRef(SnapshotId::new([5; 32]));
    let updated = original.with_output(b"second rendering".to_vec(), next_state);

    assert_eq!(original.rendered.bytes, b"first rendering");
    assert_eq!(updated.rendered.bytes, b"second rendering");
    assert_eq!(updated.rendered.base_state, next_state);
    assert_eq!(updated.rendered.cache_key.base_state, next_state);
    assert_eq!(updated.rendered.style, original.rendered.style);
    assert_eq!(updated.rendered.renderer, original.rendered.renderer);
    assert_eq!(updated.adapter, original.adapter);
}

/// Keeps a read-only fidelity assessment attached to the exact baseline.
#[test]
fn fidelity_is_bound_to_the_baseline_assessment() {
    let fidelity = FidelityAssessment::conservative("unmodeled comments");
    let baseline = ReconciliationBaseline::from_render(
        &request(SnapshotId::new([6; 32])),
        None,
        b"read only".to_vec(),
        adapter(),
        fidelity.clone(),
    );

    assert_eq!(baseline.fidelity, fidelity);
}

/// Accepts optional adapter evidence only for the exact render it describes.
#[test]
fn projection_evidence_is_bound_to_exact_render_inputs_and_output() {
    let request = request(SnapshotId::new([7; 32]));
    let rendered =
        sr_projection::RenderedProjection::from_bytes(b"const value = 1;".to_vec(), &request, None);
    let evidence = ProjectionEvidence::new(&rendered, b"source map".to_vec())
        .expect("digest-verified output accepts evidence");
    let baseline = ReconciliationBaseline::new_with_evidence(
        rendered.clone(),
        adapter(),
        FidelityAssessment::lossless(),
        Some(evidence.clone()),
    )
    .expect("matching evidence is accepted");

    assert_eq!(
        baseline.evidence().expect("binding is valid"),
        Some(&evidence)
    );

    let mut tampered = baseline.clone();
    tampered.rendered.bytes.push(b'!');
    assert_eq!(
        tampered
            .evidence()
            .expect_err("changed bytes invalidate evidence"),
        "rendered baseline digest does not match exact output bytes"
    );

    let different_output =
        sr_projection::RenderedProjection::from_bytes(b"const value = 2;".to_vec(), &request, None);
    assert_eq!(
        ReconciliationBaseline::new_with_evidence(
            different_output,
            adapter(),
            FidelityAssessment::lossless(),
            Some(evidence),
        )
        .expect_err("evidence must not transfer across outputs"),
        "projection evidence does not match exact render inputs and output"
    );

    let advanced = baseline.with_output(
        b"const value = 3;".to_vec(),
        SemanticStateRef(SnapshotId::new([8; 32])),
    );
    assert!(advanced.evidence().expect("no stale evidence").is_none());
}
