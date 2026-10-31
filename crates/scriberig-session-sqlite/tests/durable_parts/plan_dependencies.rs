//! Verifies durable plan order, dependencies, and assignments after reopen.

use super::*;

/// Verifies durable plans preserve dependency results and model assignments.
#[test]
#[allow(clippy::too_many_lines)]
fn plan_dependencies_and_assignments_survive_reopen() {
    let path =
        std::env::temp_dir().join(format!("scriberig-plan-session-{}.sqlite", EntityId::new()));
    let store = SqliteSessionStore::open(&path).expect("open store");
    let root = EntityId::from_bytes([31; 16]);
    let first = EntityId::from_bytes([32; 16]);
    let second = EntityId::from_bytes([33; 16]);
    let downstream = EntityId::from_bytes([34; 16]);
    let result = EntityId::from_bytes([35; 16]);
    let worker = EntityId::from_bytes([37; 16]);
    let candidate_context = scriberig_session::BranchCandidateContext {
        project: scriberig_core::ProjectId::from_bytes([39; 16]),
        base_snapshot: scriberig_core::SnapshotId::from_bytes([40; 32]),
        candidate_snapshot: scriberig_core::SnapshotId::from_bytes([41; 32]),
    };
    let branch_result = scriberig_session::BranchResult {
        id: result,
        branch: worker,
        status: BranchStatus::Completed,
        summary: Some("first result".to_owned()),
        result_kind: None,
        candidate_context: Some(candidate_context),
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    let mut plan = Plan {
        id: EntityId::from_bytes([36; 16]),
        source_snapshot: None,
        session: Some(EntityId::from_bytes([38; 16])),
        anchor: Some(scriberig_session::ContinuationAnchor::BranchHead { branch: root }),
        goal: "durable plan".to_owned(),
        revision: 0,
        status: scriberig_session::PlanStatus::Running,
        root_branch: root,
        nodes: vec![
            PlanNode {
                id: first,
                parent: Some(scriberig_session::PlanNodeParent::Branch(root)),
                title: "first".to_owned(),
                purpose: "first work".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                status: PlanNodeStatus::Running,
                branch: Some(worker),
                result: None,
                superseded_by: None,
                abandon_reason: None,
                last_failure: None,
                assignment: Some(ModelAssignment {
                    provider_id: "provider-a".to_owned(),
                    model_id: "model-a".to_owned(),
                    thinking_level: Some("deep".to_owned()),
                }),
                claim: Some(scriberig_session::WorkerClaim {
                    id: EntityId::from_bytes([39; 16]),
                    node: first,
                    branch: worker,
                    owner: "worker-a".to_owned(),
                    generation: 1,
                    assignment: Some(ModelAssignment {
                        provider_id: "provider-a".to_owned(),
                        model_id: "model-a".to_owned(),
                        thinking_level: Some("deep".to_owned()),
                    }),
                }),
                claim_generation: 1,
            },
            PlanNode {
                id: downstream,
                parent: Some(scriberig_session::PlanNodeParent::Branch(root)),
                title: "downstream".to_owned(),
                purpose: "dependent work".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                status: PlanNodeStatus::Planned,
                branch: None,
                result: None,
                superseded_by: None,
                abandon_reason: None,
                last_failure: None,
                assignment: Some(ModelAssignment {
                    provider_id: "provider-b".to_owned(),
                    model_id: "model-b".to_owned(),
                    thinking_level: None,
                }),
                claim: None,
                claim_generation: 0,
            },
            PlanNode {
                id: second,
                parent: Some(scriberig_session::PlanNodeParent::Branch(root)),
                title: "second".to_owned(),
                purpose: "parallel work".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                status: PlanNodeStatus::Ready,
                branch: None,
                result: None,
                superseded_by: None,
                abandon_reason: None,
                last_failure: None,
                assignment: None,
                claim: None,
                claim_generation: 0,
            },
        ],
        dependencies: Vec::new(),
        summaries: Vec::new(),
        evidence: Vec::new(),
        gates: Vec::new(),
        coordinator: Some(first),
        coordinator_cursor: Some(first),
        coordinator_claim: Some(scriberig_session::CoordinatorClaim {
            id: EntityId::from_bytes([40; 16]),
            operation: None,
            owner: "coordinator-a".to_owned(),
            generation: 1,
        }),
        coordinator_generation: 1,
        root_nodes: vec![first, downstream, second],
        history: Vec::new(),
    };
    plan.add_dependency(first, downstream)
        .expect("add dependency");
    plan.complete_node(first, EntityId::from_bytes([39; 16]), &branch_result)
        .expect("complete first");
    let mut worker_branch = Branch::new(worker, "first worker").expect("worker branch");
    worker_branch.status = BranchStatus::Completed;
    worker_branch.result = Some(branch_result.clone());
    let session = Session {
        id: EntityId::from_bytes([38; 16]),
        branches: vec![Branch::new(root, "root").expect("root"), worker_branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };

    store.save(&session).expect("save session");
    drop(store);
    let store = SqliteSessionStore::open(&path).expect("reopen store");
    let loaded = store
        .load(session.id)
        .expect("load session")
        .expect("session exists");
    let loaded_plan = &loaded.plans[0];
    assert_eq!(loaded_plan.status, scriberig_session::PlanStatus::Running);
    assert_eq!(loaded_plan.root_nodes, vec![first, downstream, second]);
    assert!(loaded_plan.coordinator_claim.is_some());
    assert_eq!(loaded_plan.nodes[0].status, PlanNodeStatus::Completed);
    assert_eq!(loaded_plan.nodes[1].status, PlanNodeStatus::Ready);
    assert_eq!(loaded_plan.dependencies[0].fulfilled_by, Some(result));
    let result = loaded.branches[1].result.as_ref().expect("durable result");
    assert_eq!(result.candidate_context, Some(candidate_context));
    assert_eq!(
        loaded_plan.ready_nodes_from_results(std::slice::from_ref(result)),
        vec![second, downstream]
    );
    let resumed_branch = EntityId::from_bytes([41; 16]);
    let mut resumed_plan = loaded_plan.clone();
    resumed_plan
        .start_node(
            downstream,
            resumed_branch,
            EntityId::from_bytes([42; 16]),
            "recovered-worker",
            std::slice::from_ref(result),
        )
        .expect("start dependent from reloaded result");
    assert_eq!(
        resumed_plan
            .nodes
            .iter()
            .find(|node| node.id == downstream)
            .unwrap()
            .status,
        PlanNodeStatus::Running
    );
    let loaded_assignment = loaded_plan.nodes[0].assignment.as_ref().unwrap();
    assert_eq!(loaded_assignment.provider_id, "provider-a");
    assert_eq!(loaded_assignment.model_id, "model-a");
    assert_eq!(loaded_plan.coordinator, Some(first));
    assert!(loaded.branches.iter().all(|branch| {
        branch
            .model_invocations
            .iter()
            .all(|invocation| invocation.request.is_none())
    }));

    let expected_revision = loaded_plan.revision;
    let mut changed = loaded.clone();
    changed.plans[0]
        .transition(scriberig_session::PlanStatus::Paused)
        .expect("pause plan");
    store
        .compare_and_swap_plan(&changed, loaded_plan.id, expected_revision)
        .expect("atomic plan update");
    assert!(matches!(
        store.compare_and_swap_plan(&loaded, loaded_plan.id, expected_revision),
        Err(scriberig_session::PlanStoreError::RevisionConflict { .. })
    ));
    let reloaded = store
        .load(session.id)
        .expect("load updated session")
        .expect("session remains present");
    assert_eq!(
        reloaded.plans[0].status,
        scriberig_session::PlanStatus::Paused
    );
    assert_eq!(
        reloaded.plans[0].history.last().unwrap().status,
        scriberig_session::PlanStatus::Paused
    );
    assert!(reloaded.branches[1].result.is_some());
    drop(store);
    let _ = std::fs::remove_file(path);
}
