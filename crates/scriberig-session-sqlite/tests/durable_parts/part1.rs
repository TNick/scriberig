//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies plan compare and swap merges new worker branch.
#[test]
fn plan_compare_and_swap_merges_new_worker_branch() {
    let session_id = EntityId::from_bytes([31; 16]);
    let root_id = EntityId::from_bytes([32; 16]);
    let worker_id = EntityId::from_bytes([33; 16]);
    let plan_id = EntityId::from_bytes([34; 16]);
    let store = SqliteSessionStore::open_in_memory().expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "run worker",
    )
    .expect("plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("planned");
    let initial_revision = plan.revision;
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial graph");
    let mut proposed = store
        .load(session_id)
        .expect("load graph")
        .expect("stored graph");
    proposed.plans[0]
        .transition(scriberig_session::PlanStatus::Running)
        .expect("start plan");
    proposed.branches.push(
        Branch::new_child(worker_id, root_id, "worker", "perform work").expect("worker branch"),
    );
    proposed.branches[0].children.push(worker_id);

    store
        .compare_and_swap_plan(&proposed, plan_id, initial_revision)
        .expect("atomically persist plan and worker branch");

    let stored = store
        .load(session_id)
        .expect("reload graph")
        .expect("stored graph");
    assert_eq!(
        stored.plans[0].status,
        scriberig_session::PlanStatus::Running
    );
    assert_eq!(stored.branches.len(), 2);
    assert_eq!(stored.branches[0].children, vec![worker_id]);
    assert_eq!(stored.branches[1].parent, Some(root_id));
}

/// Verifies selection compare-and-swap preserves concurrent branch updates.
#[test]
fn execution_selection_compare_and_swap_is_atomic() {
    let session_id = EntityId::from_bytes([11; 16]);
    let root_id = EntityId::from_bytes([12; 16]);
    let child_id = EntityId::from_bytes([13; 16]);
    let store = Arc::new(SqliteSessionStore::open_in_memory().expect("open store"));
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial session");
    let mut proposed = store
        .load(session_id)
        .expect("load selection snapshot")
        .expect("session exists");
    proposed
        .execution
        .replace(
            0,
            Some("profile-a".to_owned()),
            Some("model-a".to_owned()),
            Some("careful".to_owned()),
        )
        .expect("propose selection");
    proposed
        .execution
        .history
        .push(scriberig_session::SessionExecutionChange {
            revision: proposed.execution.revision,
            kind: scriberig_session::AgentActivityKind::ModelSelectionChanged,
            summary: "Model selected: model-a".to_owned(),
            model: proposed.execution.model.clone(),
            thinking_level: proposed.execution.thinking_level.clone(),
        });
    let mut branch_update = store
        .load(session_id)
        .expect("load branch snapshot")
        .expect("session exists");
    branch_update
        .branches
        .push(Branch::new(child_id, "new branch").expect("child"));
    store.save(&branch_update).expect("save concurrent branch");
    branch_update
        .execution
        .replace(
            0,
            Some("profile-b".to_owned()),
            Some("model-b".to_owned()),
            Some("steady".to_owned()),
        )
        .expect("propose competing selection");

    let first_store = Arc::clone(&store);
    let first =
        std::thread::spawn(move || first_store.compare_and_swap_execution_selection(&proposed, 0));
    let second_store = Arc::clone(&store);
    let second = std::thread::spawn(move || {
        second_store.compare_and_swap_execution_selection(&branch_update, 0)
    });
    let outcomes = [
        first.join().expect("first mutation"),
        second.join().expect("second mutation"),
    ];
    assert_eq!(outcomes.iter().filter(|value| value.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| matches!(value, Err(SelectionStoreError::RevisionConflict { .. })))
            .count(),
        1
    );
    let stored = store
        .load(session_id)
        .expect("reload session")
        .expect("session exists");
    assert_eq!(stored.branches.len(), 2);
    assert!(matches!(
        stored.execution.model.as_deref(),
        Some("model-a" | "model-b")
    ));
    assert_eq!(stored.execution.history.len(), 1);
    assert_eq!(
        stored.execution.history[0].model.as_deref(),
        stored.execution.model.as_deref()
    );
}

/// Proves concurrent coordinator claims serialize to one durable winner.
#[test]
#[allow(clippy::too_many_lines)]
fn concurrent_plan_coordinator_claims_have_one_winner() {
    let session_id = EntityId::from_bytes([211; 16]);
    let root_id = EntityId::from_bytes([212; 16]);
    let plan_id = EntityId::from_bytes([213; 16]);
    let store = Arc::new(SqliteSessionStore::open_in_memory().expect("open store"));
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "coordinate work",
    )
    .expect("plan");
    let node_id = EntityId::from_bytes([216; 16]);
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(root_id)),
        title: "worker".to_owned(),
        purpose: "isolated work".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    })
    .expect("add worker node");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan plan");
    plan.transition(scriberig_session::PlanStatus::Running)
        .expect("run plan");
    let initial_revision = plan.revision;
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial plan");
    let mut first = store
        .load(session_id)
        .expect("load first snapshot")
        .expect("session");
    let mut second = first.clone();
    first.plans[0]
        .claim_coordinator(EntityId::from_bytes([214; 16]), "runtime-a")
        .expect("first claim");
    second.plans[0]
        .claim_coordinator(EntityId::from_bytes([215; 16]), "runtime-b")
        .expect("second claim");

    let first_store = Arc::clone(&store);
    let first_claim = std::thread::spawn(move || {
        first_store.compare_and_swap_plan(&first, plan_id, initial_revision)
    });
    let second_store = Arc::clone(&store);
    let second_claim = std::thread::spawn(move || {
        second_store.compare_and_swap_plan(&second, plan_id, initial_revision)
    });
    let outcomes = [
        first_claim.join().expect("first operation"),
        second_claim.join().expect("second operation"),
    ];

    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| matches!(
                result,
                Err(scriberig_session::PlanStoreError::RevisionConflict { .. })
            ))
            .count(),
        1
    );
    let stored = store
        .load(session_id)
        .expect("reload plan")
        .expect("session");
    assert!(stored.plans[0].coordinator_claim.is_some());

    let expected_worker_revision = stored.plans[0].revision;
    let worker_branch = EntityId::from_bytes([217; 16]);
    let mut first_worker = stored.clone();
    let mut second_worker = stored;
    first_worker.plans[0]
        .start_node(
            node_id,
            worker_branch,
            EntityId::from_bytes([218; 16]),
            "worker-a",
            &[],
        )
        .expect("first worker claim");
    second_worker.plans[0]
        .start_node(
            node_id,
            worker_branch,
            EntityId::from_bytes([219; 16]),
            "worker-b",
            &[],
        )
        .expect("second worker claim proposal");
    let first_store = Arc::clone(&store);
    let first_claim = std::thread::spawn(move || {
        first_store.compare_and_swap_plan(&first_worker, plan_id, expected_worker_revision)
    });
    let second_store = Arc::clone(&store);
    let second_claim = std::thread::spawn(move || {
        second_store.compare_and_swap_plan(&second_worker, plan_id, expected_worker_revision)
    });
    let outcomes = [
        first_claim.join().expect("first worker operation"),
        second_claim.join().expect("second worker operation"),
    ];

    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| matches!(
                result,
                Err(scriberig_session::PlanStoreError::RevisionConflict { .. })
            ))
            .count(),
        1
    );
    let stored = store
        .load(session_id)
        .expect("reload worker claim")
        .expect("session");
    assert_eq!(stored.plans[0].nodes[0].branch, Some(worker_branch));
    assert!(stored.plans[0].nodes[0].claim.is_some());
}
