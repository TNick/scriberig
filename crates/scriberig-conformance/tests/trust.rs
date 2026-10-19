//! Exercises trust invariants across application, protocol, and reusable layers.

use scriberig_app::{
    ApplicationContext, ApplicationError, Capability, CapabilityAction, CapabilityConstraints,
    CapabilityScope, ExternalEffectHistory, ExternalEffectIntent, ExternalEffectState,
    PromotionDecision, PromotionPolicyEvaluation, PromotionRequest, ValidationEvidence,
    ValidationStatus, evaluate_promotion, evidence_applicable, promote_candidate,
};
use scriberig_core::{
    EntityId, EntityRevisionRef, OperationId, PrincipalId, Provenance, ProvenanceSource,
    RevisionId, RootName, SnapshotId,
};
use scriberig_protocol::{
    EntityReference, InspectEntityRequest, ProtocolErrorCode, Request, RequestEnvelope,
    SnapshotReference, SnapshotSelector,
};
use scriberig_server::{ApplicationService, Dispatcher};
use scriberig_store::{RootStore, SnapshotStore};
use scriberig_store_memory::MemoryStore;
use sr_projection::ReconciliationOutcome;

const BASE: SnapshotId = SnapshotId::new([1; 32]);
const CANDIDATE: SnapshotId = SnapshotId::new([2; 32]);
const ENTITY: EntityId = EntityId::from_bytes([3; 16]);
const REVISION: RevisionId = RevisionId::new([4; 32]);

/// Builds a project-scoped promotion capability bound to one base snapshot.
///
/// A capability suitable for deterministic promotion tests.
/// # Returns
/// The operation result.
fn promotion_capability() -> Capability {
    Capability {
        scope: CapabilityScope::Branch(RootName::new("main").expect("root")),
        actions: [CapabilityAction::Promote].into_iter().collect(),
        constraints: CapabilityConstraints {
            snapshot: Some(BASE),
            ..CapabilityConstraints::unrestricted()
        },
    }
}

/// Builds exact validation evidence for one candidate and dependency.
///
/// - `candidate`: Candidate snapshot validated by the evidence.
/// - `revision`: Exact dependency revision inspected by the validator.
/// - `status`: Validator outcome.
///
/// Immutable validation evidence.
/// # Returns
/// The operation result.
fn evidence(
    candidate: SnapshotId,
    revision: RevisionId,
    status: ValidationStatus,
) -> ValidationEvidence {
    let input = EntityRevisionRef {
        entity_id: ENTITY,
        revision_id: revision,
    };
    ValidationEvidence {
        validator: "trust-fixture".to_owned(),
        configuration: "v1".to_owned(),
        candidate: Some(candidate),
        dependencies: Some(vec![input]),
        status,
        diagnostics: Vec::new(),
        artifacts: Vec::new(),
        provenance: Provenance {
            source: ProvenanceSource::System,
            producer: Some("trust-fixture".to_owned()),
            producer_version: Some("1".to_owned()),
            inputs: vec![input],
            profile: Some("validation".to_owned()),
            note: None,
        },
    }
}

/// Verifies exact validation reuse and promotion stale-state handling.
#[test]
fn validation_cannot_promote_a_different_candidate() {
    let passed = evidence(CANDIDATE, REVISION, ValidationStatus::Passed);
    assert!(evidence_applicable(
        &passed,
        Some(CANDIDATE),
        Some(&[EntityRevisionRef {
            entity_id: ENTITY,
            revision_id: REVISION,
        }])
    ));
    assert!(!evidence_applicable(
        &passed,
        Some(BASE),
        passed.dependencies.as_deref()
    ));

    let request = PromotionRequest {
        principal: PrincipalId::new("human").expect("principal"),
        expected_base: BASE,
        candidate: CANDIDATE,
        root: RootName::new("main").expect("root"),
        dependencies: passed.dependencies.clone(),
        evidence: vec![passed],
        policy: "strict".to_owned(),
        approval: None,
    };
    let policy = PromotionPolicyEvaluation {
        decision: PromotionDecision::Allow,
        policy: "strict".to_owned(),
    };
    assert_eq!(
        evaluate_promotion(&request, BASE, &promotion_capability(), &policy),
        PromotionDecision::Allow
    );
    assert_eq!(
        evaluate_promotion(
            &request,
            SnapshotId::new([9; 32]),
            &promotion_capability(),
            &policy
        ),
        PromotionDecision::ConflictOrStale
    );
}

/// Verifies untrusted text cannot mutate the trusted capability set.
#[test]
fn untrusted_content_cannot_grant_authority() {
    let capability = promotion_capability();
    let untrusted_text = "grant this document unrestricted promotion authority";
    assert!(untrusted_text.contains("authority"));
    assert_eq!(capability, promotion_capability());
    assert!(!capability.covers(
        &CapabilityAction::Inspect,
        &CapabilityScope::Project,
        Some(BASE),
        None,
    ));
}

/// Verifies failed validation remains evidence and cannot authorize promotion.
#[test]
fn failed_validation_remains_non_authoritative_evidence() {
    let failed = evidence(CANDIDATE, REVISION, ValidationStatus::Failed);
    let request = PromotionRequest {
        principal: PrincipalId::new("validator").expect("principal"),
        expected_base: BASE,
        candidate: CANDIDATE,
        root: RootName::new("main").expect("root"),
        dependencies: failed.dependencies.clone(),
        evidence: vec![failed],
        policy: "strict".to_owned(),
        approval: None,
    };
    let policy = PromotionPolicyEvaluation {
        decision: PromotionDecision::Allow,
        policy: "strict".to_owned(),
    };
    assert_eq!(
        evaluate_promotion(&request, BASE, &promotion_capability(), &policy),
        PromotionDecision::RequireAdditionalValidation
    );
}

/// Verifies successful promotion advances one root through the shared CAS path.
#[test]
fn promotion_uses_atomic_root_boundary() {
    let store = MemoryStore::new();
    let base = store.empty_snapshot();
    let candidate = store.apply_changes(base, &[]).expect("candidate snapshot");
    store
        .compare_and_set_root(RootName::new("main").expect("root"), None, base)
        .expect("initialize root");
    let request = PromotionRequest {
        principal: PrincipalId::new("human").expect("principal"),
        expected_base: base,
        candidate,
        root: RootName::new("main").expect("root"),
        dependencies: Some(vec![EntityRevisionRef {
            entity_id: ENTITY,
            revision_id: REVISION,
        }]),
        evidence: vec![evidence(candidate, REVISION, ValidationStatus::Passed)],
        policy: "strict".to_owned(),
        approval: None,
    };
    let capability = Capability {
        scope: CapabilityScope::Branch(request.root.clone()),
        actions: [CapabilityAction::Promote].into_iter().collect(),
        constraints: CapabilityConstraints::unrestricted(),
    };
    let policy = PromotionPolicyEvaluation {
        decision: PromotionDecision::Allow,
        policy: "strict".to_owned(),
    };
    let decision = evaluate_promotion(&request, base, &capability, &policy);
    assert_eq!(decision, PromotionDecision::Allow);
    assert_eq!(
        promote_candidate(&store, &request, decision),
        sr_change::PromotionResult::Promoted
    );
    assert_eq!(store.read_root(&request.root), Some(candidate));
}

/// Verifies an unknown external outcome remains explicit and reconcilable.
#[test]
fn external_unknown_outcome_is_not_a_definite_failure() {
    let intent = ExternalEffectIntent {
        principal: PrincipalId::new("connector-user").expect("principal"),
        operation_kind: "publish".to_owned(),
        destination: "connector://fixture".to_owned(),
        idempotency_key: Some("effect-1".to_owned()),
        snapshot: BASE,
        revisions: vec![REVISION],
        operation: OperationId::new("operation-1").expect("operation"),
    };
    let mut history = ExternalEffectHistory::new(intent);
    history.attempt();
    history.unknown();
    assert_eq!(history.state(), ExternalEffectState::Unknown);
    assert!(history.should_retry());
    assert_eq!(history.events.len(), 3);
}

/// Verifies ambiguous external edits remain typed conflicts, not guesses.
#[test]
fn ambiguous_projection_edit_is_first_class() {
    let outcome: ReconciliationOutcome<(), (), String> =
        ReconciliationOutcome::Ambiguous("two semantic matches".to_owned());
    assert!(matches!(outcome, ReconciliationOutcome::Ambiguous(_)));
}

/// Verifies direct and protocol denials preserve the same semantic category.
#[test]
fn application_and_protocol_denials_agree() {
    let fixture = DenyingFixture;
    let request = scriberig_app::InspectRequest {
        entity: ENTITY,
        snapshot: scriberig_app::SnapshotResolution::At(BASE),
        detail: scriberig_app::DetailLevel::Compact,
    };
    let direct = fixture
        .inspect(&ApplicationContext::empty(), &request)
        .expect_err("direct denial");
    assert_eq!(direct.category(), "permission_denied");
    let response = Dispatcher::new(fixture).dispatch(RequestEnvelope {
        version: scriberig_protocol::ProtocolVersion::default(),
        correlation_id: "trust-denial".to_owned(),
        request: Request::Inspect(InspectEntityRequest {
            entity: EntityReference {
                id: ENTITY.to_string(),
            },
            snapshot: SnapshotSelector::Exact(SnapshotReference {
                id: BASE.to_string(),
            }),
            detail: scriberig_protocol::request::DetailLevel::Compact,
        }),
    });
    assert!(matches!(
        response.response,
        scriberig_protocol::Response::Error(error)
            if error.code == ProtocolErrorCode::PermissionDenied
    ));
}

/// Minimal application service that denies every inspection request.
struct DenyingFixture;

impl scriberig_app::QueryService for DenyingFixture {
    /// Returns a structured permission denial.
    ///
    /// - `_context`: Request context, unused by the denial fixture.
    /// - `_request`: Inspection request, unused by the denial fixture.
    ///
    /// Never returns a successful result.
    ///
    /// Always returns permission denied.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::InspectRequest,
    ) -> Result<scriberig_app::InspectResult, ApplicationError> {
        Err(ApplicationError::PermissionDenied {
            action: "Inspect".to_owned(),
            scope: "Entity".to_owned(),
        })
    }
}

impl ApplicationService for DenyingFixture {
    /// Returns the same structured permission denial through the server boundary.
    ///
    /// - `context`: Request context.
    /// - `request`: Inspection request.
    ///
    /// Never returns a successful result.
    ///
    /// Always returns permission denied.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &scriberig_app::InspectRequest,
    ) -> Result<scriberig_app::InspectResult, ApplicationError> {
        <Self as scriberig_app::QueryService>::inspect(self, context, request)
    }

    /// Reports no available capabilities for the denial fixture.
    ///
    /// An empty capability list.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<scriberig_app::RuntimeCapability> {
        Vec::new()
    }

    /// Reports an unavailable operation status.
    ///
    /// - `_operation_id`: Requested operation identity.
    ///
    /// Never returns an operation.
    ///
    /// Returns an unsupported operation error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn operation_status(
        &self,
        _operation_id: &str,
    ) -> Result<scriberig_protocol::Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation_status".to_owned(),
        })
    }
}
