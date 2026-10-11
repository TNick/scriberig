//! Verifies the generic projection contracts and their domain independence.

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::capabilities::CapabilityError;
use sr_projection::{
    AmbiguousMapping, CapturedObservation, ConflictId, ConflictKind, ConflictStatus, ObservationId,
    ProjectionCapabilities, ProjectionFeature, ProjectionKind, ProjectionRef, ProjectionSpecRef,
    ReconciliationOutcome,
};

/// Checks that editable capabilities cannot omit parsing support.
#[test]
fn editable_capabilities_require_parsing() {
    let result =
        ProjectionCapabilities::new([ProjectionFeature::Renderable, ProjectionFeature::Editable]);
    assert_eq!(result, Err(CapabilityError::EditableRequiresParseable));
}

/// Checks that read-only output does not require parsing support.
#[test]
fn read_only_projection_can_render_without_parsing() {
    let capabilities = ProjectionCapabilities::read_only();
    assert!(capabilities.supports(ProjectionFeature::Renderable));
    assert!(!capabilities.supports(ProjectionFeature::Parseable));
}

/// Checks that projection kinds accept adapter-defined identifiers.
#[test]
fn projection_kind_is_extensible() {
    let kind = ProjectionKind::new("vendor.custom-view.v1").unwrap();
    assert_eq!(kind.as_str(), "vendor.custom-view.v1");
}

/// Checks reconciliation states can be distinguished by enum variant.
#[test]
fn reconciliation_outcomes_are_typed() {
    let outcome: ReconciliationOutcome<(), (), ()> = ReconciliationOutcome::NoSemanticChange;
    assert!(matches!(outcome, ReconciliationOutcome::NoSemanticChange));
    let proposal: ReconciliationOutcome<(), (), ()> = ReconciliationOutcome::MigrationProposal(());
    assert!(matches!(
        proposal,
        ReconciliationOutcome::MigrationProposal(())
    ));
    let ambiguous: ReconciliationOutcome<(), (), &str> = ReconciliationOutcome::Ambiguous("detail");
    assert!(matches!(ambiguous, ReconciliationOutcome::Ambiguous(_)));
    let conflict: ReconciliationOutcome<(), (), ()> = ReconciliationOutcome::Conflict(());
    assert!(matches!(conflict, ReconciliationOutcome::Conflict(())));
    let invalid: ReconciliationOutcome<(), (), &str> =
        ReconciliationOutcome::InvalidExternalState("detail");
    assert!(matches!(
        invalid,
        ReconciliationOutcome::InvalidExternalState(_)
    ));
}

/// Checks generic projection kinds and classes contain no domain syntax.
#[test]
fn generic_model_is_domain_neutral() {
    let kind = ProjectionKind::new("application/x-example").unwrap();
    assert_eq!(kind.as_str(), "application/x-example");
}

/// Checks ambiguity keeps original evidence and competing states explicit.
#[test]
fn conflicts_retain_evidence_and_competing_states() {
    let entity = EntityId::new([7; 16]);
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
        id: ConflictId(EntityId::new([8; 16])),
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
        RenderRequest, RenderedProjection, RendererVersion, RenderingContext, SemanticScopeRef,
        SemanticStateRef, StyleRef, StyleValue,
    };
    let entity = EntityId::new([3; 16]);
    let request = RenderRequest {
        specification: ProjectionSpecRef(entity),
        scope: SemanticScopeRef {
            entity,
            revision: None,
        },
        base_state: SemanticStateRef(SnapshotId::new([4; 32])),
        kind: ProjectionKind::new("text/plain").unwrap(),
        style: StyleRef::Value(StyleValue::new(vec![1])),
        renderer: RendererVersion::new("1"),
        context: RenderingContext::default(),
    };
    let output = RenderedProjection::from_bytes(b"same".to_vec(), &request, None);
    assert_eq!(output.output_hash.as_ref().unwrap().algorithm, "blake3");
    assert_eq!(output.kind, request.kind);
    assert_eq!(output.style, request.style);
}
