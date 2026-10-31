//! Verifies adapter defaults in the projection contract.

use super::baseline;
use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    DiagnosticKind, ProjectionAdapter, ProjectionCapabilities, ProjectionDiagnostic,
    ProjectionKind, ProjectionResult, ProjectionSpecRef, ReconciliationContext,
    ReconciliationOutcome, RenderRequest, RenderedProjection, SemanticScopeRef, SemanticStateRef,
};

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
        /// - `_kind`: Projection kind being inspected.
        ///
        /// Read-only capabilities.
        /// # Returns
        /// The operation result.
        fn capabilities(&self, _kind: &ProjectionKind) -> ProjectionCapabilities {
            ProjectionCapabilities::read_only()
        }

        /// Produces an empty rendered representation for the test adapter.
        ///
        /// - `request`: Exact render request.
        ///
        /// An empty rendered projection.
        ///
        /// This test adapter never returns an error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
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
        baseline: baseline(
            SemanticStateRef(SnapshotId::new([1; 32])),
            kind.clone(),
            SemanticScopeRef {
                entity: EntityId::from_bytes([3; 16]),
                revision: None,
            },
            ProjectionSpecRef(EntityId::from_bytes([4; 16])),
        ),
        current: SemanticStateRef(SnapshotId::new([2; 32])),
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
