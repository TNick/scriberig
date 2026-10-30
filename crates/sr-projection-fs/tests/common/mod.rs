//! Supplies exact reconciliation baselines shared by filesystem integration tests.

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    FidelityAssessment, ProjectionAdapterIdentity, ProjectionKind, ProjectionRef,
    ProjectionSpecRef, ReconciliationBaseline, RenderRequest, RendererVersion, RenderingContext,
    ResolvedStyle, SemanticScopeRef, SemanticStateRef, StyleValue,
};

/// Builds a deterministic baseline for one filesystem projection test.
///
/// - `kind`: Projection kind name.
/// - `snapshot`: Exact semantic state represented by the output.
/// - `style`: Exact domain style bytes.
/// - `bytes`: Exact rendered output.
///
/// # Returns
/// A digest-verified baseline with stable test identities.
pub fn baseline(
    kind: &str,
    snapshot: SnapshotId,
    style: &[u8],
    bytes: &[u8],
) -> ReconciliationBaseline {
    let request = RenderRequest {
        specification: ProjectionSpecRef(EntityId::from_bytes([4; 16])),
        scope: SemanticScopeRef {
            entity: EntityId::from_bytes([5; 16]),
            revision: None,
        },
        base_state: SemanticStateRef(snapshot),
        kind: ProjectionKind::new(kind).expect("kind is valid"),
        style: ResolvedStyle::inline(StyleValue::new(style.to_vec())),
        renderer: RendererVersion::new("test-renderer"),
        context: RenderingContext::default(),
    };
    ReconciliationBaseline::from_render(
        &request,
        Some(ProjectionRef(EntityId::from_bytes([1; 16]))),
        bytes.to_vec(),
        ProjectionAdapterIdentity::new("test-adapter", "1").expect("adapter identity"),
        FidelityAssessment::lossless(),
    )
}
