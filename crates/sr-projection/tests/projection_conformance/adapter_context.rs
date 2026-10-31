//! Verifies adapter context in the projection contract.

use super::baseline;
use scriberig_core::{EntityId, SnapshotId};
use sr_projection::{
    ProjectionAdapter, ProjectionCapabilities, ProjectionDiagnostic, ProjectionKind,
    ProjectionResult, ProjectionSpecRef, ReconciliationContext, ReconciliationOutcome,
    RenderRequest, RenderedProjection, SemanticScopeRef, SemanticStateRef,
};

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
        /// - `_kind`: Projection kind supplied by the caller.
        ///
        /// Read-only capabilities.
        /// # Returns
        /// The operation result.
        fn capabilities(&self, _kind: &ProjectionKind) -> ProjectionCapabilities {
            ProjectionCapabilities::read_only()
        }

        /// Renders an empty representation for the test adapter.
        ///
        /// - `request`: Exact render request.
        ///
        /// An empty rendered representation.
        ///
        /// This test adapter does not return render errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn render(&self, request: &RenderRequest) -> ProjectionResult<RenderedProjection> {
            Ok(RenderedProjection::from_bytes(Vec::new(), request, None))
        }

        /// Accepts any representation for the test adapter.
        ///
        /// - `_kind`: Projection kind supplied by the caller.
        /// - `_bytes`: External representation bytes.
        ///
        /// An empty candidate.
        ///
        /// This test adapter does not return parse errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn parse(&self, _kind: &ProjectionKind, _bytes: &[u8]) -> ProjectionResult<()> {
            Ok(())
        }

        /// Verifies every synchronization context field before returning a no-op.
        ///
        /// - `_candidate`: Parsed candidate.
        /// - `context`: Synchronization context to verify.
        ///
        /// A no-semantic-change outcome.
        /// # Returns
        /// The operation result.
        fn reconcile(
            &self,
            _candidate: (),
            context: &ReconciliationContext,
        ) -> ReconciliationOutcome<(), (), Box<ProjectionDiagnostic>> {
            assert_eq!(
                context.baseline.rendered.base_state,
                SemanticStateRef(SnapshotId::new([9; 32]))
            );
            assert_eq!(context.current, SemanticStateRef(SnapshotId::new([10; 32])));
            assert_eq!(
                context.baseline.rendered.cache_key.scope.entity,
                EntityId::from_bytes([11; 16])
            );
            assert_eq!(
                context.baseline.rendered.cache_key.specification,
                ProjectionSpecRef(EntityId::from_bytes([12; 16]))
            );
            ReconciliationOutcome::NoSemanticChange
        }

        /// Accepts the test candidate.
        ///
        /// - `_candidate`: Parsed candidate.
        ///
        /// Successful validation.
        ///
        /// This test adapter does not return validation errors.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn validate(&self, _candidate: &()) -> ProjectionResult<()> {
            Ok(())
        }
    }

    let adapter = ContextAdapter;
    let context = ReconciliationContext {
        baseline: baseline(
            SemanticStateRef(SnapshotId::new([9; 32])),
            ProjectionKind::new("text/plain").unwrap(),
            SemanticScopeRef {
                entity: EntityId::from_bytes([11; 16]),
                revision: None,
            },
            ProjectionSpecRef(EntityId::from_bytes([12; 16])),
        ),
        current: SemanticStateRef(SnapshotId::new([10; 32])),
    };
    assert!(matches!(
        adapter.reconcile((), &context),
        ReconciliationOutcome::NoSemanticChange
    ));
}
