//! Verifies worker results commit with their semantic plan settlement.

use scriberig_core::EntityId;
use scriberig_session::{
    Branch, BranchResult, BranchStatus, ContinuationAnchor, ExecutionActivity, Plan, PlanNode,
    PlanNodeParent, PlanNodeStatus, PlanStatus, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;

/// Persists the accepted result and completed node through one plan CAS.
#[test]
#[allow(clippy::too_many_lines)] // One CAS scenario verifies both persisted views.
fn plan_settlement_commits_existing_branch_result_with_semantic_state() {
    let session_id = EntityId::from_bytes([101; 16]);
    let root_branch = EntityId::from_bytes([102; 16]);
    let worker_branch = EntityId::from_bytes([103; 16]);
    let plan_id = EntityId::from_bytes([104; 16]);
    let node_id = EntityId::from_bytes([105; 16]);
    let coordinator_id = EntityId::from_bytes([106; 16]);
    let claim_id = EntityId::from_bytes([107; 16]);
    let result_id = EntityId::from_bytes([108; 16]);
    let store = SqliteSessionStore::open_in_memory().expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
        "settle worker atomically",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(root_branch)),
        title: "worker".to_owned(),
        purpose: "complete focused work".to_owned(),
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
    .expect("add node");
    plan.transition(PlanStatus::Planned).expect("plan plan");
    plan.start_with_coordinator_claim(coordinator_id, "coordinator", None)
        .expect("start plan");
    plan.start_nodes(&[(node_id, worker_branch, claim_id)], "worker", &[])
        .expect("claim worker");
    let result = BranchResult {
        id: result_id,
        branch: worker_branch,
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
    };
    let expected_revision = plan.revision;
    let mut initial = Session {
        id: session_id,
        branches: vec![
            Branch::new(root_branch, "root").expect("root"),
            Branch::new_child(worker_branch, root_branch, "worker", "work").expect("worker branch"),
        ],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    initial.branches[0].children.push(worker_branch);
    store.save(&initial).expect("save active claim");

    let mut proposed = store
        .load(session_id)
        .expect("load before settlement")
        .expect("stored session");
    proposed.plans[0]
        .settle_workers(
            &[(node_id, claim_id, result.clone())],
            &[],
            std::slice::from_ref(&result),
        )
        .expect("settle plan node");
    let branch = proposed
        .branches
        .iter_mut()
        .find(|branch| branch.id == worker_branch)
        .expect("worker branch");
    branch.result = Some(result.clone());
    branch.status = BranchStatus::Completed;
    branch
        .activity
        .push(ExecutionActivity::BranchResult(result_id));

    store
        .compare_and_swap_plan(&proposed, plan_id, expected_revision)
        .expect("atomically commit plan result");
    let stored = store
        .load(session_id)
        .expect("reload settlement")
        .expect("stored session");
    let branch = stored
        .branches
        .iter()
        .find(|branch| branch.id == worker_branch)
        .expect("durable worker branch");
    assert_eq!(branch.result.as_ref(), Some(&result));
    assert_eq!(branch.status, BranchStatus::Completed);
    assert!(
        branch
            .activity
            .contains(&ExecutionActivity::BranchResult(result_id))
    );
    assert_eq!(stored.plans[0].status, PlanStatus::Completed);
    assert_eq!(stored.plans[0].nodes[0].result, Some(result_id));
}
