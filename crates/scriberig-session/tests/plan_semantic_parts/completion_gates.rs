//! Verifies completion gates block plan completion until satisfied or waived.

use scriberig_core::EntityId;
use scriberig_session::{
    BranchResult, BranchStatus, ContinuationAnchor, Plan, PlanEvidenceKind, PlanGate,
    PlanGatePhase, PlanGateRequirement, PlanGateStatus, PlanGateWaiver, PlanNode, PlanNodeParent,
    PlanNodeStatus, PlanStatus,
};

/// Blocks completed nodes from completing their plan while a gate is pending.
#[test]
fn pending_completion_gate_blocks_plan_completion_until_waived() {
    let session = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let plan_id = EntityId::from_bytes([3; 16]);
    let parent_id = EntityId::from_bytes([4; 16]);
    let child_id = EntityId::from_bytes([5; 16]);
    let gate_id = EntityId::from_bytes([6; 16]);
    let mut plan = Plan::new(
        plan_id,
        session,
        ContinuationAnchor::BranchHead { branch },
        "complete only after gate review",
    )
    .expect("plan");
    plan.add_node(completed_node(parent_id, PlanNodeParent::Branch(branch)))
        .expect("parent node");
    plan.add_node(completed_node(child_id, PlanNodeParent::Node(parent_id)))
        .expect("child node");
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: parent_id,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::Artifact,
        },
        waiver: None,
    })
    .expect("completion gate");
    plan.transition(PlanStatus::Planned).expect("planned");
    plan.transition(PlanStatus::Running).expect("running");
    let results = [result(7, branch), result(8, branch)];

    assert_eq!(
        plan.evaluate_gate(gate_id).expect("gate evaluation").status,
        PlanGateStatus::Pending
    );
    assert!(
        !plan
            .complete_if_satisfied(&results)
            .expect("pending gate check")
    );
    assert_eq!(plan.status, PlanStatus::Running);

    plan.waive_gate(
        gate_id,
        PlanGateWaiver {
            reason: "review accepted the remaining child work".to_owned(),
            actor: None,
            operation: None,
        },
    )
    .expect("waive completion gate");

    assert_eq!(
        plan.evaluate_gate(gate_id)
            .expect("waived evaluation")
            .status,
        PlanGateStatus::Waived
    );
    assert!(
        plan.complete_if_satisfied(&results)
            .expect("waived gate permits completion")
    );
    assert_eq!(plan.status, PlanStatus::Completed);
}

/// Creates a completed node with an exact result branch.
///
/// # Parameters
/// - `id`: Stable node identity.
/// - `parent`: Structural parent.
///
/// # Returns
/// A completed plan node bound to a deterministic branch and result.
fn completed_node(id: EntityId, parent: PlanNodeParent) -> PlanNode {
    PlanNode {
        id,
        parent: Some(parent),
        title: "completed work".to_owned(),
        purpose: "provide a durable result".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Completed,
        branch: Some(EntityId::from_bytes([2; 16])),
        result: Some(EntityId::from_bytes([id.as_bytes()[0] + 3; 16])),
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    }
}

/// Creates a matching successful branch result.
///
/// # Parameters
/// - `id`: Result identity byte repeated across the ID.
/// - `branch`: Branch carrying the result.
///
/// # Returns
/// A successful result with no additional payload.
///
/// # Errors
/// This helper constructs a value directly and cannot fail.
fn result(id: u8, branch: EntityId) -> BranchResult {
    BranchResult {
        id: EntityId::from_bytes([id; 16]),
        branch,
        status: BranchStatus::Completed,
        summary: None,
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    }
}
