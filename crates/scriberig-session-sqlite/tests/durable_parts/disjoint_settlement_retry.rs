//! Verifies deterministic retry for concurrent disjoint worker settlements.

use scriberig_core::EntityId;
use scriberig_session::{
    Branch, BranchResult, BranchStatus, ContinuationAnchor, Plan, PlanNode, PlanNodeParent,
    PlanNodeStatus, PlanStatus, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One end-to-end regression covers stale settlement and recovery."
)]
/// Retries a stale sibling settlement without losing either result.
fn disjoint_worker_settlements_conflict_then_retry_without_losing_results() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-disjoint-settlement-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([121; 16]);
    let root_id = EntityId::from_bytes([122; 16]);
    let left_branch = EntityId::from_bytes([123; 16]);
    let right_branch = EntityId::from_bytes([124; 16]);
    let plan_id = EntityId::from_bytes([125; 16]);
    let left_node = EntityId::from_bytes([126; 16]);
    let right_node = EntityId::from_bytes([127; 16]);
    let coordinator = EntityId::from_bytes([128; 16]);
    let left_claim = EntityId::from_bytes([129; 16]);
    let right_claim = EntityId::from_bytes([130; 16]);
    let left_result = make_branch_result(EntityId::from_bytes([131; 16]), left_branch);
    let right_result = make_branch_result(EntityId::from_bytes([132; 16]), right_branch);
    let first_store = SqliteSessionStore::open(&path).expect("open first store");
    let second_store = SqliteSessionStore::open(&path).expect("open second store");

    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: root_id },
        "settle disjoint siblings",
    )
    .expect("plan");
    plan.add_node(node(left_node, left_branch))
        .expect("left node");
    plan.add_node(node(right_node, right_branch))
        .expect("right node");
    plan.transition(PlanStatus::Planned).expect("plan plan");
    plan.start_with_coordinator_claim(coordinator, "coordinator", None)
        .expect("start plan");
    plan.start_nodes(
        &[
            (left_node, left_branch, left_claim),
            (right_node, right_branch, right_claim),
        ],
        "workers",
        &[],
    )
    .expect("claim sibling workers");
    let expected = plan.revision;
    let session = Session {
        id: session_id,
        branches: vec![
            Branch::new(root_id, "root").expect("root branch"),
            Branch::new_child(left_branch, root_id, "left", "left worker").expect("left branch"),
            Branch::new_child(right_branch, root_id, "right", "right worker")
                .expect("right branch"),
        ],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    first_store.save(&session).expect("save claimed plan");

    let left = settled_proposal(
        &first_store,
        session_id,
        left_node,
        left_claim,
        &left_result,
    );
    let right = settled_proposal(
        &second_store,
        session_id,
        right_node,
        right_claim,
        &right_result,
    );
    first_store
        .compare_and_swap_plan(&left, plan_id, expected)
        .expect("first sibling settlement wins");
    let conflict = second_store
        .compare_and_swap_plan(&right, plan_id, expected)
        .expect_err("stale sibling settlement conflicts");
    assert!(matches!(
        conflict,
        scriberig_session::PlanStoreError::RevisionConflict {
            expected: actual_expected,
            actual
        } if actual_expected == expected && actual == expected + 1
    ));

    let retried = settled_proposal(
        &second_store,
        session_id,
        right_node,
        right_claim,
        &right_result,
    );
    second_store
        .compare_and_swap_plan(&retried, plan_id, expected + 1)
        .expect("retry second sibling against the current revision");
    let stored = first_store
        .load(session_id)
        .expect("load final state")
        .expect("stored session");
    assert_eq!(stored.plans[0].nodes[0].result, Some(left_result.id));
    assert_eq!(stored.plans[0].nodes[1].result, Some(right_result.id));
    assert_eq!(stored.plans[0].status, PlanStatus::Running);
    assert!(
        stored.plans[0]
            .nodes
            .iter()
            .all(|node| { node.status == PlanNodeStatus::Completed })
    );
    assert_eq!(
        stored
            .branches
            .iter()
            .filter(|branch| branch.result.is_some())
            .count(),
        2
    );
    drop(first_store);
    drop(second_store);
    let _ = std::fs::remove_file(path);
}

/// Builds one completed worker proposal from the latest stored plan revision.
///
/// # Parameters
/// - `store`: Session repository containing the active worker claim.
/// - `session_id`: Owning session identity.
/// - `node_id`: Claimed plan node to settle.
/// - `claim_id`: Exact active worker claim.
/// - `result`: Accepted result to persist with the settlement.
///
/// # Returns
/// Updated session projection suitable for one plan-local compare-and-swap.
fn settled_proposal(
    store: &SqliteSessionStore,
    session_id: EntityId,
    node_id: EntityId,
    claim_id: EntityId,
    result: &BranchResult,
) -> Session {
    let mut proposal = store
        .load(session_id)
        .expect("load active plan")
        .expect("stored plan");
    proposal.plans[0]
        .settle_workers(
            &[(node_id, claim_id, result.clone())],
            &[],
            std::slice::from_ref(result),
        )
        .expect("settle worker");
    let branch = proposal
        .branches
        .iter_mut()
        .find(|branch| branch.id == result.branch)
        .expect("worker branch");
    branch.result = Some(result.clone());
    branch.status = BranchStatus::Completed;
    branch
        .activity
        .push(scriberig_session::ExecutionActivity::BranchResult(
            result.id,
        ));
    proposal
}

/// Creates a minimal successful branch result for one worker.
///
/// # Parameters
/// - `id`: Stable result identity.
/// - `branch`: Worker branch that produced the result.
///
/// # Returns
/// A completed result with no handles or validation payloads.
///
/// # Errors
/// This fixture constructor does not fail.
fn make_branch_result(id: EntityId, branch: EntityId) -> BranchResult {
    BranchResult {
        id,
        branch,
        status: BranchStatus::Completed,
        summary: Some("worker completed".to_owned()),
        result_kind: Some("summary".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    }
}

/// Creates one branch-owned planned worker node.
///
/// # Parameters
/// - `id`: Stable plan-node identity.
/// - `branch`: Worker branch owned by the node.
///
/// # Returns
/// A planned node ready for coordinator dispatch.
fn node(id: EntityId, branch: EntityId) -> PlanNode {
    PlanNode {
        id,
        parent: Some(PlanNodeParent::Branch(branch)),
        title: "worker".to_owned(),
        purpose: "complete disjoint work".to_owned(),
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
