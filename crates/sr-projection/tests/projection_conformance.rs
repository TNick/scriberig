//! Verifies the generic projection contracts and their domain independence.

use sr_projection::{
    FidelityAssessment, ProjectionAdapterIdentity, ProjectionKind, ProjectionSpecRef,
    ReconciliationBaseline, RenderRequest, RenderedProjection, RendererVersion, RenderingContext,
    ResolvedStyle, SemanticScopeRef, SemanticStateRef, StyleValue,
};

/// Builds exact rendered baseline evidence for projection contract tests.
///
/// - `base`: Semantic state represented by the baseline.
/// - `kind`: Projection kind represented by the baseline.
/// - `scope`: Exact semantic scope represented by the baseline.
/// - `specification`: Persistent projection specification.
///
/// # Returns
/// A typed baseline with deterministic test bytes.
fn baseline(
    base: SemanticStateRef,
    kind: ProjectionKind,
    scope: SemanticScopeRef,
    specification: ProjectionSpecRef,
) -> ReconciliationBaseline {
    let request = RenderRequest {
        specification,
        scope,
        base_state: base,
        kind,
        style: ResolvedStyle::inline(StyleValue::new(Vec::new())),
        renderer: RendererVersion::new("test-renderer"),
        context: RenderingContext::default(),
    };
    ReconciliationBaseline::new(
        RenderedProjection::from_bytes(b"test-baseline".to_vec(), &request, None),
        ProjectionAdapterIdentity::new("test-adapter", "1").expect("adapter identity"),
        FidelityAssessment::lossless(),
    )
    .expect("baseline digest matches rendered bytes")
}

#[path = "projection_conformance/adapter_context.rs"]
mod adapter_context;
#[path = "projection_conformance/adapter_defaults.rs"]
mod adapter_defaults;
#[path = "projection_conformance/capabilities.rs"]
mod capabilities;
#[path = "projection_conformance/identity.rs"]
mod identity;
