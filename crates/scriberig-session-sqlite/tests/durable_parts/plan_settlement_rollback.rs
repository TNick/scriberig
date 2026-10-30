//! Verifies a failed plan promotion rolls back worker result projections.

use super::*;

/// Rolls back a settled worker result when semantic plan promotion fails.
#[test]
#[allow(clippy::too_many_lines)] // This exercises the full settlement rollback boundary.
fn plan_cas_rolls_back_session_projection_on_semantic_conflict() {
    let path =
        std::env::temp_dir().join(format!("scriberig-plan-atomic-{}.sqlite", EntityId::new()));
    let session_id = EntityId::from_bytes([65; 16]);
    let root_id = EntityId::from_bytes([66; 16]);
    let worker_id = EntityId::from_bytes([67; 16]);
    let plan_id = EntityId::from_bytes([68; 16]);
    let node_id = EntityId::from_bytes([69; 16]);
    let coordinator_id = EntityId::from_bytes([70; 16]);
    let claim_id = EntityId::from_bytes([71; 16]);
    let result_id = EntityId::from_bytes([72; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "atomic plan settlement",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(root_id)),
        title: "worker".to_owned(),
        purpose: "complete work".to_owned(),
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
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan plan");
    plan.start_with_coordinator_claim(coordinator_id, "coordinator", None)
        .expect("start plan");
    let claim_revision = plan.revision;
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save running plan");
    let mut claimed = store
        .load(session_id)
        .expect("load running plan")
        .expect("stored session");
    claimed.plans[0]
        .start_nodes(&[(node_id, worker_id, claim_id)], "worker", &[])
        .expect("claim worker");
    claimed.branches.push(
        Branch::new_child(worker_id, root_id, "worker", "complete work").expect("worker branch"),
    );
    claimed.branches[0].children.push(worker_id);
    store
        .compare_and_swap_plan(&claimed, plan_id, claim_revision)
        .expect("persist active claim");

    let connection = rusqlite::Connection::open(&path).expect("open trigger connection");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_plan_root_update
             BEFORE UPDATE ON roots
             BEGIN SELECT RAISE(ABORT, 'injected promotion failure'); END;",
        )
        .expect("install root promotion failure");
    drop(connection);

    let mut proposal = store
        .load(session_id)
        .expect("load active claim")
        .expect("stored session");
    let expected = proposal.plans[0].revision;
    let result = BranchResult {
        id: result_id,
        branch: worker_id,
        status: BranchStatus::Completed,
        summary: Some("worker completed".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    proposal.plans[0]
        .settle_workers(
            &[(node_id, claim_id, result.clone())],
            &[],
            std::slice::from_ref(&result),
        )
        .expect("settle worker");
    let worker = proposal
        .branches
        .iter_mut()
        .find(|branch| branch.id == worker_id)
        .expect("worker branch");
    worker.result = Some(result);
    worker.status = BranchStatus::Completed;
    worker
        .activity
        .push(scriberig_session::ExecutionActivity::BranchResult(
            result_id,
        ));

    assert!(
        store
            .compare_and_swap_plan(&proposal, plan_id, expected)
            .is_err()
    );
    let stored = store
        .load(session_id)
        .expect("reload session after failed promotion")
        .expect("stored session");
    let worker = stored
        .branches
        .iter()
        .find(|branch| branch.id == worker_id)
        .expect("stored worker branch");
    assert_eq!(stored.plans[0].nodes[0].status, PlanNodeStatus::Running);
    assert_eq!(worker.status, BranchStatus::Active);
    assert!(worker.result.is_none());
    assert_eq!(stored.plans[0].nodes[0].result, None);
    drop(store);
    let _ = std::fs::remove_file(path);
}
