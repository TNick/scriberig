//! Verifies plan retry guards use exact dependencies and gate evidence.

use scriberig_core::EntityId;
use scriberig_session::{
    BranchStatus, ContinuationAnchor, Plan, PlanEvidenceAssociation, PlanEvidenceKind,
    PlanEvidenceOwner, PlanEvidenceReference, PlanEvidenceRole, PlanGate, PlanGatePhase,
    PlanGateRequirement, PlanNode, PlanNodeParent, PlanNodeStatus, PlanSemanticReference,
    PlanStatus,
};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The regression exercises retry guards as one ordered lifecycle scenario."
)]
/// Rejects blocked or claimed retries and accepts one after exact evidence.
fn retry_requires_satisfied_dependencies_gates_and_no_claim() {
    let session = EntityId::from_bytes([1; 16]);
    let root = EntityId::from_bytes([2; 16]);
    let prerequisite = EntityId::from_bytes([3; 16]);
    let dependent = EntityId::from_bytes([4; 16]);
    let prerequisite_branch = EntityId::from_bytes([5; 16]);
    let dependent_branch = EntityId::from_bytes([6; 16]);
    let result_id = EntityId::from_bytes([7; 16]);
    let gate_id = EntityId::from_bytes([8; 16]);
    let evidence_id = EntityId::from_bytes([9; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([10; 16]),
        session,
        ContinuationAnchor::BranchHead { branch: root },
        "recover blocked work",
    )
    .expect("plan");
    plan.add_node(node(prerequisite, root, "prerequisite"))
        .expect("add prerequisite");
    plan.add_node(node(dependent, root, "dependent"))
        .expect("add dependent");
    plan.add_dependency(prerequisite, dependent)
        .expect("dependency");
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: dependent,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::Artifact,
        },
        waiver: None,
    })
    .expect("entry gate");
    plan.transition(PlanStatus::Planned).expect("plan plan");
    plan.start_with_coordinator_claim(EntityId::from_bytes([11; 16]), "coordinator", None)
        .expect("start plan");
    plan.nodes[0].status = PlanNodeStatus::Failed;
    plan.nodes[0].branch = Some(prerequisite_branch);
    plan.nodes[1].status = PlanNodeStatus::Failed;
    plan.nodes[1].branch = Some(dependent_branch);
    let failed_prerequisite = scriberig_session::BranchResult {
        id: EntityId::from_bytes([12; 16]),
        branch: prerequisite_branch,
        status: BranchStatus::Abandoned,
        summary: Some("prerequisite failed".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };

    assert!(
        plan.retry_failed_node(dependent, &[failed_prerequisite])
            .is_err()
    );
    assert_eq!(plan.nodes[1].status, PlanNodeStatus::Failed);

    let successful_prerequisite = scriberig_session::BranchResult {
        id: result_id,
        branch: prerequisite_branch,
        status: BranchStatus::Completed,
        summary: Some("prerequisite completed".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    plan.nodes[0].status = PlanNodeStatus::Completed;
    plan.nodes[0].result = Some(result_id);
    plan.dependencies[0].fulfilled_by = Some(result_id);
    assert!(
        plan.retry_failed_node(dependent, std::slice::from_ref(&successful_prerequisite))
            .is_err()
    );
    assert_eq!(plan.nodes[1].status, PlanNodeStatus::Failed);

    plan.attach_evidence(PlanEvidenceAssociation {
        id: evidence_id,
        owner: PlanEvidenceOwner::Gate(gate_id),
        role: PlanEvidenceRole::Supports,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::Artifact,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([13; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::default(),
        },
        note: None,
        actor: None,
    })
    .expect("satisfy entry gate");
    plan.retry_failed_node(dependent, std::slice::from_ref(&successful_prerequisite))
        .expect("retry after requirements are satisfied");
    assert_eq!(plan.nodes[1].status, PlanNodeStatus::Ready);
    assert_eq!(plan.nodes[1].branch, None);

    plan.nodes[1].status = PlanNodeStatus::Failed;
    plan.nodes[1].branch = Some(dependent_branch);
    plan.nodes[1].claim = Some(scriberig_session::WorkerClaim {
        id: EntityId::from_bytes([14; 16]),
        node: dependent,
        branch: dependent_branch,
        owner: "stale-worker".to_owned(),
        generation: 1,
        assignment: None,
    });
    assert!(
        plan.retry_failed_node(dependent, std::slice::from_ref(&successful_prerequisite))
            .is_err()
    );
}

/// Creates one unstarted node under a session branch.
///
/// # Parameters
/// - `id`: Stable node identity.
/// - `parent`: Session branch used as the structural parent.
/// - `title`: Node title and purpose.
///
/// # Returns
/// A planned node without branch, result, or worker claim.
fn node(id: EntityId, parent: EntityId, title: &str) -> PlanNode {
    PlanNode {
        id,
        parent: Some(PlanNodeParent::Branch(parent)),
        title: title.to_owned(),
        purpose: title.to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    }
}
