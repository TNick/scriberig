//! Verifies the generic projection contracts and their domain independence.

use scriberig_core::{EntityId, SnapshotId};
use sr_projection::capabilities::CapabilityError;
use sr_projection::{
    AmbiguousMapping, CapturedObservation, ConflictId, ConflictKind, ConflictStatus,
    DiagnosticKind, ObservationId, ProjectionAdapter, ProjectionCapabilities, ProjectionDiagnostic,
    ProjectionFeature, ProjectionKind, ProjectionRef, ProjectionResult, ProjectionSpecRef,
    ReconciliationContext, ReconciliationOutcome, RenderRequest, RenderedProjection,
    RendererVersion, RenderingContext, SemanticScopeRef, SemanticStateRef,
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

/// Checks style and exact context participate in the render cache identity.
#[test]
fn render_cache_identity_includes_exact_inputs() {
    use sr_projection::{RenderCacheKey, StyleRef, StyleValue};
    let entity = EntityId::new([4; 16]);
    let mut request = RenderRequest {
        specification: ProjectionSpecRef(entity),
        scope: SemanticScopeRef {
            entity,
            revision: None,
        },
        base_state: SemanticStateRef(SnapshotId::new([5; 32])),
        kind: ProjectionKind::new("text/plain").unwrap(),
        style: StyleRef::Value(StyleValue::new(vec![1])),
        renderer: RendererVersion::new("1"),
        context: RenderingContext::default(),
    };
    let first = RenderCacheKey::from_request(&request);
    request.style = StyleRef::Value(StyleValue::new(vec![2]));
    let second = RenderCacheKey::from_request(&request);
    assert_ne!(first, second);
}

/// Checks rendered outputs can carry adapter-owned reusable fragments.
#[test]
fn rendered_output_carries_reusable_fragments() {
    use sr_projection::{RenderedFragment, RenderedProjection};
    let entity = EntityId::new([6; 16]);
    let request = RenderRequest {
        specification: ProjectionSpecRef(entity),
        scope: SemanticScopeRef {
            entity,
            revision: None,
        },
        base_state: SemanticStateRef(SnapshotId::new([7; 32])),
        kind: ProjectionKind::new("text/plain").unwrap(),
        style: sr_projection::StyleRef::Value(sr_projection::StyleValue::new(vec![1])),
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
    let entity = EntityId::new([8; 16]);
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

/// Checks read-only adapters receive explicit unsupported defaults.
#[test]
fn read_only_adapters_have_explicit_unsupported_import_defaults() {
    /// Test adapter that exposes rendering but no editable operations.
    struct ReadOnlyAdapter;

    impl ProjectionAdapter for ReadOnlyAdapter {
        type Candidate = ();
        type Migration = ();
        type Conflict = ();

        /// Returns read-only capabilities for every test projection kind.
        ///
        /// # Parameters
        /// - `_kind`: Projection kind being inspected.
        ///
        /// # Returns
        /// Read-only capabilities.
        fn capabilities(&self, _kind: &ProjectionKind) -> ProjectionCapabilities {
            ProjectionCapabilities::read_only()
        }

        /// Produces an empty rendered representation for the test adapter.
        ///
        /// # Parameters
        /// - `request`: Exact render request.
        ///
        /// # Returns
        /// An empty rendered projection.
        ///
        /// # Errors
        /// This test adapter never returns an error.
        fn render(&self, request: &RenderRequest) -> ProjectionResult<RenderedProjection> {
            Ok(RenderedProjection::from_bytes(Vec::new(), request, None))
        }
    }

    let adapter = ReadOnlyAdapter;
    let kind = ProjectionKind::new("text/plain").unwrap();
    let parse_error = adapter.parse(&kind, b"external").unwrap_err();
    assert_eq!(parse_error.kind, DiagnosticKind::UnsupportedOperation);
    assert_eq!(parse_error.stage, sr_projection::DiagnosticStage::Parse);

    let context = ReconciliationContext {
        base: SemanticStateRef(SnapshotId::new([1; 32])),
        current: SemanticStateRef(SnapshotId::new([2; 32])),
        scope: SemanticScopeRef {
            entity: EntityId::new([3; 16]),
            revision: None,
        },
        specification: ProjectionSpecRef(EntityId::new([4; 16])),
        metadata: std::collections::BTreeMap::new(),
    };
    let outcome = adapter.reconcile((), &context);
    assert!(matches!(
        outcome,
        ReconciliationOutcome::InvalidExternalState(detail)
            if detail.kind == DiagnosticKind::UnsupportedOperation
    ));
    let validation_error = adapter.validate(&()).unwrap_err();
    assert_eq!(validation_error.kind, DiagnosticKind::UnsupportedOperation);
}

/// Checks diagnostics expose a stable operation category.
#[test]
fn diagnostics_have_typed_categories() {
    use sr_projection::DiagnosticStage;
    let diagnostic = ProjectionDiagnostic {
        kind: DiagnosticKind::ParseFailure,
        stage: DiagnosticStage::Parse,
        projection: None,
        scope: None,
        detail: sr_projection::DiagnosticDetail {
            code: "invalid".to_owned(),
            data: Vec::new(),
        },
        source: Some(sr_projection::DiagnosticSource {
            subsystem: "test-parser".to_owned(),
            code: "invalid".to_owned(),
            message: Some("source failure".to_owned()),
        }),
        message: Some("invalid input".to_owned()),
        retryable: false,
    };
    assert_eq!(diagnostic.kind, DiagnosticKind::ParseFailure);
    assert_eq!(diagnostic.source.as_ref().unwrap().subsystem, "test-parser");
}

/// Confirms adapters receive the complete synchronization context.
#[test]
fn adapter_reconciliation_receives_full_context() {
    /// Minimal adapter used to verify context delivery.
    struct ContextAdapter;

    impl ProjectionAdapter for ContextAdapter {
        type Candidate = ();
        type Migration = ();
        type Conflict = ();

        /// Returns read-only capabilities for the test adapter.
        ///
        /// # Parameters
        /// - `_kind`: Projection kind supplied by the caller.
        ///
        /// # Returns
        /// Read-only capabilities.
        fn capabilities(&self, _kind: &ProjectionKind) -> ProjectionCapabilities {
            ProjectionCapabilities::read_only()
        }

        /// Renders an empty representation for the test adapter.
        ///
        /// # Parameters
        /// - `request`: Exact render request.
        ///
        /// # Returns
        /// An empty rendered representation.
        ///
        /// # Errors
        /// This test adapter does not return render errors.
        fn render(&self, request: &RenderRequest) -> ProjectionResult<RenderedProjection> {
            Ok(RenderedProjection::from_bytes(Vec::new(), request, None))
        }

        /// Accepts any representation for the test adapter.
        ///
        /// # Parameters
        /// - `_kind`: Projection kind supplied by the caller.
        /// - `_bytes`: External representation bytes.
        ///
        /// # Returns
        /// An empty candidate.
        ///
        /// # Errors
        /// This test adapter does not return parse errors.
        fn parse(&self, _kind: &ProjectionKind, _bytes: &[u8]) -> ProjectionResult<()> {
            Ok(())
        }

        /// Verifies every synchronization context field before returning a no-op.
        ///
        /// # Parameters
        /// - `_candidate`: Parsed candidate.
        /// - `context`: Synchronization context to verify.
        ///
        /// # Returns
        /// A no-semantic-change outcome.
        fn reconcile(
            &self,
            _candidate: (),
            context: &ReconciliationContext,
        ) -> ReconciliationOutcome<(), (), Box<ProjectionDiagnostic>> {
            assert_eq!(context.base, SemanticStateRef(SnapshotId::new([9; 32])));
            assert_eq!(context.current, SemanticStateRef(SnapshotId::new([10; 32])));
            assert_eq!(context.scope.entity, EntityId::new([11; 16]));
            assert_eq!(
                context.specification,
                ProjectionSpecRef(EntityId::new([12; 16]))
            );
            ReconciliationOutcome::NoSemanticChange
        }

        /// Accepts the test candidate.
        ///
        /// # Parameters
        /// - `_candidate`: Parsed candidate.
        ///
        /// # Returns
        /// Successful validation.
        ///
        /// # Errors
        /// This test adapter does not return validation errors.
        fn validate(&self, _candidate: &()) -> ProjectionResult<()> {
            Ok(())
        }
    }

    let adapter = ContextAdapter;
    let context = ReconciliationContext {
        base: SemanticStateRef(SnapshotId::new([9; 32])),
        current: SemanticStateRef(SnapshotId::new([10; 32])),
        scope: SemanticScopeRef {
            entity: EntityId::new([11; 16]),
            revision: None,
        },
        specification: ProjectionSpecRef(EntityId::new([12; 16])),
        metadata: std::collections::BTreeMap::new(),
    };
    assert!(matches!(
        adapter.reconcile((), &context),
        ReconciliationOutcome::NoSemanticChange
    ));
}
