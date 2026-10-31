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

/// Ensures a required decomposition adds a completion obligation to its parent.
#[test]
fn required_decomposition_blocks_parent_completion_until_child_result_exists() {
    let session = EntityId::from_bytes([11; 16]);
    let branch = EntityId::from_bytes([12; 16]);
    let plan_id = EntityId::from_bytes([13; 16]);
    let parent_id = EntityId::from_bytes([14; 16]);
    let child_id = EntityId::from_bytes([15; 16]);
    let mut plan = Plan::new(
        plan_id,
        session,
        ContinuationAnchor::BranchHead { branch },
        "complete required decomposition",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: parent_id,
        parent: Some(PlanNodeParent::Branch(branch)),
        title: "parent work".to_owned(),
        purpose: "complete parent work".to_owned(),
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
    })
    .expect("parent node");
    plan.transition(PlanStatus::Planned).expect("planned");
    plan.transition(PlanStatus::Running).expect("running");
    plan.decompose(
        parent_id,
        vec![PlanNode {
            id: child_id,
            parent: None,
            title: "required child".to_owned(),
            purpose: "complete required work".to_owned(),
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
        }],
        true,
    )
    .expect("required decomposition");
    let parent_result = result(16, branch);
    let parent = plan
        .nodes
        .iter_mut()
        .find(|node| node.id == parent_id)
        .unwrap();
    parent.status = PlanNodeStatus::Completed;
    parent.branch = Some(branch);
    parent.result = Some(parent_result.id);

    assert_eq!(plan.gates.len(), 1);
    assert_eq!(
        plan.evaluate_gate(plan.gates[0].id)
            .expect("required child gate")
            .status,
        PlanGateStatus::Pending
    );
    assert!(
        !plan
            .complete_if_satisfied(std::slice::from_ref(&parent_result))
            .expect("pending child blocks completion")
    );
    assert_eq!(plan.status, PlanStatus::Running);

    let child_result = result(17, branch);
    let child = plan
        .nodes
        .iter_mut()
        .find(|node| node.id == child_id)
        .unwrap();
    child.status = PlanNodeStatus::Completed;
    child.branch = Some(branch);
    child.result = Some(child_result.id);
    assert!(
        plan.complete_if_satisfied(&[parent_result, child_result])
            .expect("completed required child permits completion")
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
