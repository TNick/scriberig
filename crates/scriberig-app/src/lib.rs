//! Defines shared semantic operations consumed by every product surface.

pub mod capability;
pub mod capability_registry;
pub mod command;
pub mod context;
pub mod error;
pub mod event;
pub mod external_effect;
pub mod guarded_derive;
pub mod guarded_query;
mod guarded_query_support;
pub mod operation;
pub mod promotion;
pub mod query;
pub mod read_backend;
pub mod request;
pub mod result;
pub mod service;
mod service_store;
pub mod trust;
pub mod validation;

pub use capability::{
    AuthorizationDenial, BranchContext, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityDescriptor, CapabilityId, CapabilityKind, CapabilityScope, DomainOperationRequest,
    RuntimeCapability,
};
pub use capability_registry::{
    CapabilityExecution, CapabilityProvider, CapabilityRegistrationError, CapabilityRegistry,
    DuplicateCapability,
};
pub use command::{CommandContext, CommandKind};
pub use context::{ApplicationContext, CapabilityContext, Principal};
pub use error::{ApplicationError, ConflictDetails};
pub use event::ApplicationEvent;
pub use external_effect::{
    ExternalEffectEvent, ExternalEffectHistory, ExternalEffectIntent, ExternalEffectReceipt,
    ExternalEffectState, authorize_external_effect,
};
pub use guarded_derive::GuardedDerivedRepository;
pub use guarded_query::GuardedQueryService;
pub use operation::{
    OperationId, OperationKind, OperationProgress, OperationRecord, OperationStatus,
};
pub use promotion::{
    ApprovalEvidence, PromotionDecision, PromotionPolicyEvaluation, PromotionRequest,
    evaluate_promotion, promote_candidate,
};
pub use query::{DetailLevel, InspectRequest, InspectResult, QueryService, SnapshotResolution};
pub use read_backend::UniversalReadBackend;
pub use request::{
    ApplicationRequest, ApplyRequest, BranchRequest, CommitRequest, DiffRequest, HistoryRequest,
    RenderRequest, SnapshotSelector,
};
pub use result::{ApplicationResult, OperationStarted, OperationSummary};
pub use service::{ApplicationBackend, ApplicationService};
pub use service_store::StoreInspectionBackend;
pub use trust::{
    ApprovalSummary, CapabilityDenial, ConflictSummary, ExternalEffectSummary, PolicyDecision,
    PolicyEvaluation, SourceKind, TrustSummary, ValidationState,
};
pub use validation::{
    DiagnosticSeverity, ValidationCandidate, ValidationDiagnostic, ValidationEvidence,
    ValidationStatus, Validator, evidence_applicable,
};

#[cfg(test)]
mod tests {
    use super::{
        ApplicationError, ApplicationEvent, AuthorizationDenial, BranchContext, Capability,
        CapabilityAction, CapabilityConstraints, CapabilityDescriptor, CapabilityId,
        CapabilityKind, CapabilityScope, DetailLevel, DomainOperationRequest, InspectRequest,
        InspectResult, OperationKind, OperationStatus, SnapshotResolution, SnapshotSelector,
    };
    use std::collections::BTreeSet;

    use scriberig_core::{EntityId, ObjectType, PrincipalId, RevisionId, RootName, SnapshotId};
    use scriberig_query::Selection;

    /// Creates a one-action capability for a scope test.
    ///
    /// # Parameters
    /// - `scope`: Scope granted by the test capability.
    /// - `action`: Action granted by the test capability.
    ///
    /// # Returns
    /// A capability containing the requested single action.
    fn capability(scope: CapabilityScope, action: CapabilityAction) -> Capability {
        Capability {
            scope,
            actions: [action].into_iter().collect(),
            constraints: CapabilityConstraints::unrestricted(),
        }
    }

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

    /// Verifies read and branch changes do not imply promotion authority.
    #[test]
    fn capabilities_keep_mutating_actions_distinct() {
        let inspect = capability(CapabilityScope::Project, CapabilityAction::Inspect);
        let modify = capability(CapabilityScope::Project, CapabilityAction::ModifyBranch);
        let project = CapabilityScope::Project;
        assert!(!inspect.covers(&CapabilityAction::Promote, &project, None, None,));
        assert!(!modify.covers(&CapabilityAction::Promote, &project, None, None,));
    }

    /// Verifies project reads cover entities while entity reads stay narrow.
    #[test]
    fn capability_scopes_preserve_least_authority() {
        let entity = EntityId::new([10; 16]);
        let project = capability(CapabilityScope::Project, CapabilityAction::Inspect);
        let entity_read = capability(CapabilityScope::Entity(entity), CapabilityAction::Inspect);
        let relation = CapabilityScope::Relation {
            entity,
            relation: "depends_on".to_owned(),
        };
        assert!(project.covers(
            &CapabilityAction::Inspect,
            &CapabilityScope::Entity(entity),
            None,
            None,
        ));
        assert!(!entity_read.covers(&CapabilityAction::Inspect, &relation, None, None));
    }

    /// Verifies external invocation requires an explicitly named destination.
    #[test]
    fn external_invocation_requires_destination_scope() {
        let capability = capability(
            CapabilityScope::ExternalDestination("https://example.test".to_owned()),
            CapabilityAction::InvokeExternal,
        );
        assert!(!capability.covers(
            &CapabilityAction::InvokeExternal,
            &CapabilityScope::Project,
            None,
            None,
        ));
        assert!(capability.covers(
            &CapabilityAction::InvokeExternal,
            &CapabilityScope::ExternalDestination("https://example.test".to_owned()),
            None,
            None,
        ));
    }

    /// Verifies delegated capabilities cannot widen scope or add actions.
    #[test]
    fn delegation_cannot_widen_authority() {
        let parent = capability(CapabilityScope::Project, CapabilityAction::Inspect);
        let child = capability(
            CapabilityScope::Entity(EntityId::new([11; 16])),
            CapabilityAction::Inspect,
        );
        let promoted = capability(CapabilityScope::Project, CapabilityAction::Promote);
        assert!(parent.can_delegate(&child));
        assert!(!parent.can_delegate(&promoted));
        assert!(!child.can_delegate(&parent));
    }

    /// Verifies exact snapshot constraints reject other snapshots.
    #[test]
    fn snapshot_constraints_are_enforced() {
        let snapshot = SnapshotId::new([12; 32]);
        let constrained = Capability {
            scope: CapabilityScope::Project,
            actions: [CapabilityAction::Inspect].into_iter().collect(),
            constraints: CapabilityConstraints {
                snapshot: Some(snapshot),
                ..CapabilityConstraints::unrestricted()
            },
        };
        assert!(constrained.covers(
            &CapabilityAction::Inspect,
            &CapabilityScope::Project,
            Some(snapshot),
            None,
        ));
        assert!(!constrained.covers(
            &CapabilityAction::Inspect,
            &CapabilityScope::Project,
            Some(SnapshotId::new([13; 32])),
            None,
        ));
    }

    /// Verifies denials remain structured and principal identity grants nothing.
    #[test]
    fn denial_is_structured_and_identity_is_not_authority() {
        let principal = PrincipalId::new("agent-1").expect("valid principal");
        let denial = AuthorizationDenial {
            principal: Some(principal.clone()),
            action: CapabilityAction::Promote,
            scope: CapabilityScope::Project,
            reason: "no promotion capability".to_owned(),
            alternatives: vec![CapabilityAction::Inspect],
            approval_possible: true,
        };
        assert_eq!(denial.principal, Some(principal));
        assert_eq!(denial.alternatives, vec![CapabilityAction::Inspect]);
        let no_capability = Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::default(),
            constraints: CapabilityConstraints::unrestricted(),
        };
        assert!(!no_capability.covers(
            &CapabilityAction::Inspect,
            &CapabilityScope::Project,
            None,
            None,
        ));
    }
}
