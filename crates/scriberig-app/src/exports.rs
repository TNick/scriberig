//! Re-exports the application crate public contract types.

pub use crate::agent_console::preferences::{
    AgentPresentationPreferenceSnapshot, AgentPresentationPreferenceStore,
    AgentPresentationPreferences, InMemoryAgentPresentationPreferenceStore, PromptVisibility,
    ReasoningVisibility, ToolInvocationDetail, ToolOutputVisibility,
};
pub use crate::agent_console::resolution::{
    ExecutionSelectionSources, ResolvedPlanExecution, resolve_execution_selection,
    resolve_execution_selection_typed, resolve_execution_selection_with_profile_preference,
    resolve_plan_node_execution, resolve_plan_node_execution_with_profile_preference,
    resolve_plan_node_execution_with_sources,
};
pub use crate::agent_profile::types::{
    AgentProfileDescriptor, AgentProfileExecutionPreference, AgentProfileRevisionRef,
    ContextActivationRule, ContextComponentDescriptor, ContextComponentKind,
    ContextComponentRevisionRef, LegacyProfileMappingRequest,
};
pub use crate::agent_profile::{
    AgentProfileError, AgentProfileProvider, AgentProfileService, AgentProfileStoreResult,
    ProfileCandidate, ProfileInspection,
};
pub use crate::artifact::{
    ArtifactManifestPage, MAX_ARTIFACT_PAGE, SpreadsheetCellEdit, SpreadsheetCellInspection,
    SpreadsheetCellSearchRequest, SpreadsheetCells, SpreadsheetEditCandidate,
};
pub use crate::artifact_members::{ArtifactMembers, MAX_ARTIFACT_MEMBER_PAGE};
pub use crate::cad_candidate::{CadLayerCandidate, CadLineCandidate};
pub use crate::cad_provider_registry::{CadProviderRegistry, CadProviderRegistryError};
pub use crate::cad_read_query::{CadLayerEntitiesRequest, CadTextSearchRequest};
pub use crate::capability::{
    AuthorizationDenial, BranchContext, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityDescriptor, CapabilityId, CapabilityKind, CapabilityScope, DomainOperationRequest,
    RuntimeCapability,
};
pub use crate::capability_registry::{
    CapabilityExecution, CapabilityProvider, CapabilityRegistrationError, CapabilityRegistry,
    DuplicateCapability,
};
pub use crate::command::{CommandContext, CommandKind};
pub use crate::composite_backend::CompositeBackend;
pub use crate::context::{ApplicationContext, BootstrapEnvironment, CapabilityContext, Principal};
pub use crate::document_candidate::DocumentParagraphCandidate;
pub use crate::document_read::DocumentParagraphSearchRequest;
pub use crate::error::{ApplicationError, ConflictDetails};
pub use crate::event::ApplicationEvent;
pub use crate::external_effect::{
    ExternalEffectEvent, ExternalEffectHistory, ExternalEffectIntent, ExternalEffectReceipt,
    ExternalEffectState, authorize_external_effect,
};
pub use crate::geo_artifact::{GeoFeaturePage, GeoPackageCatalogPage};
pub use crate::geo_candidate::{GeoAttributeCandidate, GeoGeometryCandidate};
pub use crate::geo_query::{GeoFeatureQuery, GeoFeatureQueryResult};
pub use crate::guarded_derive::GuardedDerivedRepository;
pub use crate::guarded_query::GuardedQueryService;
pub use crate::operation::{
    OperationId, OperationKind, OperationProgress, OperationRecord, OperationRegistry,
    OperationStatus,
};
pub use crate::operation_backend::{ApplicationBackendComposition, OperationProvider};
pub use crate::operation_store::StoreOperationProvider;
pub use crate::projection_backend::ProjectionOperationProvider;
pub use crate::projection_materialization::{
    MaterializedProjection, ProjectionMaterializationRequest,
};
pub use crate::projection_style::{
    CompatibilityStyleAdapter, EffectiveProjectionStyle, InMemoryProjectionStyleStore,
    KindDispatchStyleAdapter, ProjectionStyleBackend, ProjectionStyleConfiguration,
    ProjectionStyleFidelityRequest, ProjectionStyleSchemaRequest, ProjectionStyleSettingRequest,
    ProjectionStyleStore, RepositoryConventionSource, StaticRepositoryConvention, StoredStyle,
    StyleEntryKey, StyleResolution, StyleResolutionRequest, StyleScopeKind,
};
pub use crate::promotion::{
    ApprovalEvidence, PromotionDecision, PromotionPolicyEvaluation, PromotionRequest,
    evaluate_promotion, promote_candidate,
};
pub use crate::provider_lifecycle::ProviderExtensionRuntime;
pub use crate::provider_lifecycle::{
    ModelCatalogGeneration, catalog_generation, reconcile_model_catalog,
};
pub use crate::qgis_candidate::{QgisLayerOrderCandidate, QgisLayerRenameCandidate};
pub use crate::qgis_read::{QgisProjectQuery, QgisProjectResult};
pub use crate::query::{
    DetailLevel, DocumentParagraphEditQuery, DocumentTableCellEditQuery, DocumentTreeQuery,
    InspectRequest, InspectResult, QueryService, SnapshotResolution, SpreadsheetCellEditQuery,
    SpreadsheetCellQuery, SpreadsheetCellsQuery,
};
pub use crate::read_backend::UniversalReadBackend;
pub use crate::request::{
    ApplicationRequest, ApplyRequest, BranchRequest, CommitRequest, DiffRequest, HistoryRequest,
    MergeRequest, RenderRequest, RenderStylePurpose, RenderStyleSelection, SnapshotSelector,
};
pub use crate::result::{
    ApplicationResult, MigrationDelta, MigrationDetail, OperationStarted, OperationSummary,
};
pub use crate::service::{ApplicationBackend, ApplicationService};
pub use crate::service_store::StoreInspectionBackend;
pub use crate::session::{
    AgentCommandRequest, AgentConsoleBackend, AgentContinuationAccepted, AgentContinueAtRequest,
    AgentResult, AgentSelection, CandidateChange, LegacyExecutionMappingRequest,
    LegacyPlanAssignmentMappingRequest, LocalAgentHandle, LocalChildAgentHandle,
    NewSessionAgentRequest, NewSessionAgentStart, PlanAssignmentRequest, PlanAttemptHistoryRequest,
    PlanCreateRequest, PlanLifecycleAction, PlanLifecycleRequest, PlanMutation,
    PlanMutationRequest, PlanNodeDetailsRequest, PlanNodeDraft, PlanNodeReference,
    PlanReplanRequest, PlanRetryRequest, SessionBackend, SessionModelSelectionRequest,
    SessionRequest, SessionThinkingSelectionRequest, ToolExecutionEvidence,
};
pub use crate::trust::{
    ApprovalSummary, CapabilityDenial, ConflictSummary, ExternalEffectSummary, PolicyDecision,
    PolicyEvaluation, SourceKind, TrustSummary, ValidationState,
};
pub use crate::validation::{
    DiagnosticSeverity, ValidationCandidate, ValidationDiagnostic, ValidationEvidence,
    ValidationStatus, Validator, evidence_applicable,
};
pub use scriberig_project::{
    DocumentBlockView, DocumentCellView, DocumentParagraphMatch, DocumentParagraphSearch,
    DocumentRowView, DocumentSectionView, DocumentStyleView, DocumentTableView, DocumentTreeView,
};
pub use scriberig_session::SessionExecutionSelection;

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
    /// - `scope`: Scope granted by the test capability.
    /// - `action`: Action granted by the test capability.
    ///
    /// A capability containing the requested single action.
    /// # Returns
    /// The operation result.
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
            entity: EntityId::from_bytes([1; 16]),
            revision: RevisionId::new([2; 32]),
        };
        assert_eq!(error.category(), "revision_mismatch");
        assert_ne!(error.to_string(), "");
    }

    /// Verifies events retain stable handles rather than owning canonical state.
    #[test]
    fn events_retain_snapshot_and_revision_handles() {
        let event = ApplicationEvent::EntityChanged {
            entity: EntityId::from_bytes([1; 16]),
            revision: RevisionId::new([2; 32]),
        };
        assert!(matches!(event, ApplicationEvent::EntityChanged { .. }));
        let snapshot = SnapshotId::new([3; 32]);
        assert_eq!(snapshot.as_bytes(), &[3; 32]);
    }

    /// Verifies inspection request and result types preserve snapshot identity.
    #[test]
    fn inspection_preserves_requested_snapshot() {
        let entity = EntityId::from_bytes([4; 16]);
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
            OperationKind::Agent,
            OperationKind::Plan,
            OperationKind::Apply,
            OperationKind::Commit,
            OperationKind::DomainOperation,
        ];
        assert_eq!(kinds.len(), 14);
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
        let entity = EntityId::from_bytes([8; 16]);
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
        let entity = EntityId::from_bytes([10; 16]);
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
            CapabilityScope::Entity(EntityId::from_bytes([11; 16])),
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
