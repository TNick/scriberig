//! Verifies identity in the projection contract.

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    AmbiguousMapping, CapturedObservation, ConflictId, ConflictKind, ConflictStatus, ObservationId,
    ProjectionKind, ProjectionRef, ProjectionSpecRef, RenderRequest, RendererVersion,
    RenderingContext, SemanticScopeRef, SemanticStateRef,
};

/// Checks ambiguity keeps original evidence and competing states explicit.
#[test]
fn conflicts_retain_evidence_and_competing_states() {
    let entity = EntityId::from_bytes([7; 16]);
    let observation = CapturedObservation {
        id: ObservationId(entity),
        projection: ProjectionRef(entity),
        previous_rendered_revision: None,
        observed_content: b"external".to_vec(),
        metadata: Vec::new(),
        semantic_snapshot: SnapshotId::new([1; 32]),
        adapter_id: "test".to_owned(),
        adapter_version: "1".to_owned(),
    };
    let conflict = sr_projection::ProjectionConflict {
        id: ConflictId(EntityId::from_bytes([8; 16])),
        specification: ProjectionSpecRef(entity),
        projection: ProjectionRef(entity),
        base_snapshot: SnapshotId::new([1; 32]),
        current_snapshot: SnapshotId::new([2; 32]),
        observation,
        candidate_revision: None,
        ambiguous_mappings: vec![AmbiguousMapping {
            entity,
            candidates: Vec::new(),
        }],
        conflicting_entities: vec![entity],
        kind: ConflictKind::AmbiguousMapping,
        status: ConflictStatus::Unresolved,
    };
    assert_eq!(conflict.observation.observed_content, b"external");
    assert_ne!(conflict.base_snapshot, conflict.current_snapshot);
}

/// Checks concurrent canonical changes are represented by an explicit kind.
#[test]
fn concurrent_changes_have_first_class_conflict_kind() {
    assert_eq!(
        sr_projection::ConflictKind::ConcurrentCanonicalChange,
        sr_projection::ConflictKind::ConcurrentCanonicalChange,
    );
}

/// Checks exact render inputs and output digest metadata are reproducible.
#[test]
fn deterministic_render_identity_includes_style() {
    use sr_projection::{
        RenderRequest, RenderedProjection, RendererVersion, RenderingContext, ResolvedStyle,
        SemanticScopeRef, SemanticStateRef, StyleValue,
    };
    let entity = EntityId::from_bytes([3; 16]);
    let request = RenderRequest {
        specification: ProjectionSpecRef(entity),
        scope: SemanticScopeRef {
            entity,
            revision: None,
        },
        base_state: SemanticStateRef(SnapshotId::new([4; 32])),
        kind: ProjectionKind::new("text/plain").unwrap(),
        style: ResolvedStyle::inline(StyleValue::new(vec![1])),
        renderer: RendererVersion::new("1"),
        context: RenderingContext::default(),
    };
    let output = RenderedProjection::from_bytes(b"same".to_vec(), &request, None);
    assert_eq!(output.output_hash.as_ref().unwrap().algorithm, "blake3");
    assert_eq!(output.kind, request.kind);
    assert_eq!(output.style, request.style);
}

/// Checks style and exact context participate in the render cache identity.
#[test]
fn render_cache_identity_includes_exact_inputs() {
    use sr_projection::{RenderCacheKey, ResolvedStyle, StyleValue};
    let entity = EntityId::from_bytes([4; 16]);
    let mut request = RenderRequest {
        specification: ProjectionSpecRef(entity),
        scope: SemanticScopeRef {
            entity,
            revision: None,
        },
        base_state: SemanticStateRef(SnapshotId::new([5; 32])),
        kind: ProjectionKind::new("text/plain").unwrap(),
        style: ResolvedStyle::inline(StyleValue::new(vec![1])),
        renderer: RendererVersion::new("1"),
        context: RenderingContext::default(),
    };
    let first = RenderCacheKey::from_request(&request);
    request.style = ResolvedStyle::inline(StyleValue::new(vec![2]));
    let second = RenderCacheKey::from_request(&request);
    assert_ne!(first, second);
}

/// Checks rendered outputs can carry adapter-owned reusable fragments.
#[test]
fn rendered_output_carries_reusable_fragments() {
    use sr_projection::{RenderedFragment, RenderedProjection};
    let entity = EntityId::from_bytes([6; 16]);
    let request = RenderRequest {
        specification: ProjectionSpecRef(entity),
        scope: SemanticScopeRef {
            entity,
            revision: None,
        },
        base_state: SemanticStateRef(SnapshotId::new([7; 32])),
        kind: ProjectionKind::new("text/plain").unwrap(),
        style: sr_projection::ResolvedStyle::inline(sr_projection::StyleValue::new(vec![1])),
        renderer: RendererVersion::new("1"),
        context: RenderingContext::default(),
    };
    let fragment = RenderedFragment::new(b"local-input", b"fragment");
    let output = RenderedProjection::from_bytes(b"output".to_vec(), &request, None)
        .with_fragments(vec![fragment.clone()]);
    assert_eq!(output.fragments, vec![fragment]);
}

/// Checks specifications can carry an opaque external location reference.
#[test]
fn specification_can_reference_external_location() {
    use sr_projection::{
        ExternalLocation, OwnershipClass, ProjectionClass, ProjectionSpec, RendererVersion,
    };
    let entity = EntityId::from_bytes([8; 16]);
    let specification = ProjectionSpec {
        scope: SemanticScopeRef {
            entity,
            revision: None,
        },
        kind: ProjectionKind::new("text/plain").unwrap(),
        class: ProjectionClass::Editable,
        style: sr_projection::StyleRef::Value(sr_projection::StyleValue::new(vec![])),
        adapter_id: "test".to_owned(),
        adapter_version: "1".to_owned(),
        renderer: RendererVersion::new("renderer-1"),
        configuration: std::collections::BTreeMap::new(),
        ownership: OwnershipClass::Managed,
        location: Some(ExternalLocation::new("file:///tmp/example.txt").unwrap()),
    };
    assert_eq!(
        specification.location.as_ref().unwrap().as_str(),
        "file:///tmp/example.txt"
    );
    assert_eq!(specification.renderer, RendererVersion::new("renderer-1"));
}
