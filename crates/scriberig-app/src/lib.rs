//! Defines shared semantic operations consumed by every product surface.

pub mod capability;
pub mod command;
pub mod context;
pub mod error;
pub mod event;
pub mod operation;
pub mod query;
pub mod request;
pub mod result;
pub mod trust;

pub use capability::{
    BranchContext, CapabilityDescriptor, CapabilityId, CapabilityKind, DomainOperationRequest,
    RuntimeCapability,
};
pub use command::{CommandContext, CommandKind};
pub use context::{ApplicationContext, CapabilityContext, Principal};
pub use error::ApplicationError;
pub use event::ApplicationEvent;
pub use operation::{OperationId, OperationKind, OperationProgress, OperationStatus};
pub use query::{DetailLevel, InspectRequest, InspectResult, QueryService, SnapshotResolution};
pub use request::{
    ApplicationRequest, ApplyRequest, BranchRequest, CommitRequest, DiffRequest, HistoryRequest,
    RenderRequest, SnapshotSelector,
};
pub use result::{ApplicationResult, OperationStarted, OperationSummary};
pub use trust::{
    ApprovalSummary, CapabilityDenial, ConflictSummary, ExternalEffectSummary, PolicyDecision,
    PolicyEvaluation, SourceKind, TrustSummary, ValidationState,
};

#[cfg(test)]
mod tests {
    use super::{
        ApplicationError, ApplicationEvent, BranchContext, CapabilityDescriptor, CapabilityId,
        CapabilityKind, DetailLevel, DomainOperationRequest, InspectRequest, InspectResult,
        OperationKind, OperationStatus, SnapshotResolution, SnapshotSelector,
    };
    use scriberig_core::{EntityId, ObjectType, RevisionId, RootName, SnapshotId};
    use scriberig_query::Selection;

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

    /// Verifies every universal operation has a stable operation kind.
    #[test]
    fn universal_operation_kinds_are_complete() {
        let kinds = [
            OperationKind::Inspect,
            OperationKind::List,
            OperationKind::Follow,
            OperationKind::Find,
            OperationKind::Query,
            OperationKind::Diff,
            OperationKind::History,
            OperationKind::Render,
            OperationKind::Branch,
            OperationKind::Apply,
            OperationKind::Commit,
        ];
        assert_eq!(kinds.len(), 11);
    }

    /// Verifies snapshot selectors preserve exact and named-root semantics.
    #[test]
    fn snapshot_selectors_remain_distinct() {
        let snapshot = SnapshotId::new([7; 32]);
        let root = RootName::new("main").expect("valid root");
        assert_ne!(
            SnapshotSelector::Exact(snapshot),
            SnapshotSelector::NamedRoot(root)
        );
    }

    /// Verifies capability descriptors and domain requests retain semantic handles.
    #[test]
    fn domain_operation_request_is_structured() {
        let entity = EntityId::new([8; 16]);
        let selection = Selection {
            snapshot: SnapshotId::new([9; 32]),
            entity,
            revision: None,
            relation_path: Vec::new(),
            projection_anchor: None,
        };
        let capability = CapabilityDescriptor {
            id: CapabilityId::new("command.rename").expect("valid capability"),
            kind: CapabilityKind::DomainOperation,
            label: "Rename".to_owned(),
            description: "Rename an entity".to_owned(),
            entity_types: vec![ObjectType::new("rust.function").expect("valid type")],
            available: true,
            derived: false,
            estimated_items: None,
        };
        let request = DomainOperationRequest {
            capability: capability.id.clone(),
            subject: selection,
            context: BranchContext {
                base_snapshot: SnapshotId::new([9; 32]),
                branch: None,
            },
            arguments: Vec::new(),
        };
        assert_eq!(request.capability, capability.id);
        assert_eq!(request.subject.entity, entity);
    }
}
