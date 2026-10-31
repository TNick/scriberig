//! Verifies the application-backed inspect tool delegates to the query boundary.

use scriberig_agent_core::{Tool, ToolDetail, ToolError, ToolInput};
use scriberig_app::agent_tools::{INSPECT_CAPABILITY, InspectTool};
use scriberig_app::{
    ApplicationContext, ApplicationError, CapabilityContext, CapabilityId, DetailLevel,
    InspectRequest, InspectResult, QueryService, SnapshotResolution,
};
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Records the exact request received by the fake application boundary.
struct Fixture;

impl QueryService for Fixture {
    /// Returns a deterministic result for the adapter test.
    ///
    /// - `self`: Fixture service.
    /// - `_context`: Trusted request context.
    /// - `request`: Snapshot-aware inspect request.
    ///
    /// Deterministic inspection result.
    ///
    /// Never returns an error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        assert_eq!(
            request.snapshot,
            SnapshotResolution::At(SnapshotId::new([3; 32]))
        );
        assert_eq!(request.detail, DetailLevel::Detailed);
        Ok(InspectResult {
            entity: request.entity,
            revision: Some(RevisionId::new([2; 32])),
            snapshot: Some(SnapshotId::new([3; 32])),
            object_type: None,
            label: Some("fixture".to_owned()),
            description: None,
            capabilities: vec!["handle:next".to_owned()],
            trust: None,
        })
    }
}

/// Creates a context granting the inspect capability.
///
/// Context with the inspect capability granted.
/// # Returns
/// The operation result.
fn allowed_context() -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new(INSPECT_CAPABILITY).expect("capability")
        ]),
        ..ApplicationContext::empty()
    }
}

/// Verifies input mapping, snapshot preservation, and stable handles.
#[test]
fn inspect_adapter_delegates_semantics() {
    let entity = EntityId::from_bytes([1; 16]);
    let result = InspectTool::new(Fixture)
        .invoke(
            &allowed_context(),
            ToolInput {
                entity,
                snapshot: Some(SnapshotId::new([3; 32])),
                detail: ToolDetail::Detailed,
            },
        )
        .expect("inspect");
    assert_eq!(result.entity, entity);
    assert_eq!(result.snapshot, Some(SnapshotId::new([3; 32])));
    assert_eq!(result.handles, vec!["handle:next"]);
}

/// Uses trusted exact context when the model omits its own snapshot argument.
#[test]
fn inspect_defaults_to_the_authoritative_context_snapshot() {
    let result = InspectTool::new(Fixture)
        .invoke(
            &allowed_context().with_snapshot(SnapshotId::new([3; 32])),
            ToolInput {
                entity: EntityId::from_bytes([1; 16]),
                snapshot: None,
                detail: ToolDetail::Detailed,
            },
        )
        .expect("inspect exact context");
    assert_eq!(result.snapshot, Some(SnapshotId::new([3; 32])));
}

/// Verifies capability denial occurs before the application operation.
#[test]
fn inspect_adapter_denies_without_capability() {
    let error = InspectTool::new(Fixture)
        .invoke(
            &ApplicationContext::empty(),
            ToolInput {
                entity: EntityId::from_bytes([1; 16]),
                snapshot: None,
                detail: ToolDetail::Compact,
            },
        )
        .expect_err("missing capability");
    assert!(matches!(error, ToolError::CapabilityDenied { .. }));
}

/// Verifies branch cancellation reaches the shared inspect query operation.
#[test]
fn inspect_adapter_observes_branch_cancellation() {
    let cancelled = std::sync::atomic::AtomicBool::new(true);
    let error = InspectTool::new(Fixture)
        .invoke_cancellable(
            &allowed_context(),
            ToolInput {
                entity: EntityId::from_bytes([1; 16]),
                snapshot: None,
                detail: ToolDetail::Compact,
            },
            &cancelled,
        )
        .expect_err("cancelled inspection");
    assert!(matches!(
        error,
        ToolError::Application { category, .. } if category == "operation_cancelled"
    ));
}
