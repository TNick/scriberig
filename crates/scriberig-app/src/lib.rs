//! Defines shared semantic operations consumed by every product surface.

pub mod capability;
pub mod command;
pub mod context;
pub mod error;
pub mod event;
pub mod operation;
pub mod query;
pub mod trust;

pub use capability::{CapabilityId, RuntimeCapability};
pub use command::{CommandContext, CommandKind};
pub use context::{ApplicationContext, CapabilityContext, Principal};
pub use error::ApplicationError;
pub use event::ApplicationEvent;
pub use operation::{OperationId, OperationKind, OperationProgress, OperationStatus};
pub use query::{DetailLevel, InspectRequest, InspectResult, QueryService, SnapshotResolution};
pub use trust::{
    ApprovalSummary, CapabilityDenial, ConflictSummary, ExternalEffectSummary, PolicyDecision,
    PolicyEvaluation, SourceKind, TrustSummary, ValidationState,
};

#[cfg(test)]
mod tests {
    use super::{
        ApplicationError, ApplicationEvent, DetailLevel, InspectRequest, InspectResult,
        OperationStatus, SnapshotResolution,
    };
    use scriberig_core::{EntityId, RevisionId, SnapshotId};

    /// Verifies terminal operation states remain distinguishable from active states.
    #[test]
    fn operation_status_has_explicit_terminal_states() {
        assert!(!OperationStatus::Queued.is_terminal());
        assert!(!OperationStatus::Running.is_terminal());
        assert!(OperationStatus::Completed.is_terminal());
        assert!(OperationStatus::Failed.is_terminal());
        assert!(OperationStatus::Cancelled.is_terminal());
    }

    /// Verifies application errors expose stable categories without text parsing.
    #[test]
    fn application_error_categories_are_structured() {
        let error = ApplicationError::RevisionMismatch {
            entity: EntityId::new([1; 16]),
            revision: RevisionId::new([2; 32]),
        };
        assert_eq!(error.category(), "revision_mismatch");
        assert_ne!(error.to_string(), "");
    }

    /// Verifies events retain stable handles rather than owning canonical state.
    #[test]
    fn events_retain_snapshot_and_revision_handles() {
        let event = ApplicationEvent::EntityChanged {
            entity: EntityId::new([1; 16]),
            revision: RevisionId::new([2; 32]),
        };
        assert!(matches!(event, ApplicationEvent::EntityChanged { .. }));
        let snapshot = SnapshotId::new([3; 32]);
        assert_eq!(snapshot.as_bytes(), &[3; 32]);
    }

    /// Verifies inspection request and result types preserve snapshot identity.
    #[test]
    fn inspection_preserves_requested_snapshot() {
        let entity = EntityId::new([4; 16]);
        let snapshot = SnapshotId::new([5; 32]);
        let request = InspectRequest {
            entity,
            snapshot: SnapshotResolution::At(snapshot),
            detail: DetailLevel::Compact,
        };
        let result = InspectResult {
            entity,
            revision: Some(RevisionId::new([6; 32])),
            snapshot: Some(snapshot),
            object_type: None,
            label: None,
            description: None,
            capabilities: vec!["inspect.revision".to_owned()],
            trust: None,
        };
        assert_eq!(request.snapshot, SnapshotResolution::At(snapshot));
        assert_eq!(result.snapshot, Some(snapshot));
    }
}
