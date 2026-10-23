//! Verifies session graphs survive repository reopen and preserve child state.

use scriberig_core::EntityId;
use scriberig_session::{
    Branch, BranchStatus, CacheDisposition, ModelAssignment, ModelInvocation, Plan, PlanNode,
    PlanNodeStatus, SelectionStoreError, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;
use std::sync::Arc;

/// Lists complete durable sessions in stable identity order.
#[test]
fn session_store_lists_durable_graphs() {
    let path =
        std::env::temp_dir().join(format!("scriberig-session-list-{}.sqlite", EntityId::new()));
    let store = SqliteSessionStore::open(&path).expect("open store");
    for id in [EntityId::from_bytes([2; 16]), EntityId::from_bytes([1; 16])] {
        store
            .save(&Session {
                id,
                branches: vec![Branch::new(EntityId::new(), "root").expect("root")],
                goals: Vec::new(),
                tasks: Vec::new(),
                plans: Vec::new(),
                execution: scriberig_session::SessionExecutionSelection::default(),
            })
            .expect("save session");
    }

    let sessions = store.list().expect("list sessions");

    assert_eq!(
        sessions
            .iter()
            .map(|session| session.id)
            .collect::<Vec<_>>(),
        [EntityId::from_bytes([1; 16]), EntityId::from_bytes([2; 16])]
    );
    let _ = std::fs::remove_file(path);
}

/// Removes exactly the requested durable session while retaining other sessions.
#[test]
fn session_store_removes_one_exact_graph() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-remove-{}.sqlite",
        EntityId::new()
    ));
    let store = SqliteSessionStore::open(&path).expect("open store");
    let ids = [
        EntityId::from_bytes([11; 16]),
        EntityId::from_bytes([12; 16]),
    ];
    for id in ids {
        store
            .save(&Session {
                id,
                branches: vec![Branch::new(EntityId::new(), "root").expect("root")],
                goals: Vec::new(),
                tasks: Vec::new(),
                plans: Vec::new(),
                execution: scriberig_session::SessionExecutionSelection::default(),
            })
            .expect("save session");
    }

    store.remove(ids[0]).expect("remove session");

    assert!(store.load(ids[0]).expect("load removed session").is_none());
    assert!(store.load(ids[1]).expect("load retained session").is_some());
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Verifies a complete session graph round-trips through SQLite.
#[test]
fn session_graph_survives_reopen() {
    let path = std::env::temp_dir().join(format!("scriberig-session-{}.sqlite", EntityId::new()));
    let session_id = EntityId::from_bytes([1; 16]);
    let root_id = EntityId::from_bytes([2; 16]);
    let child_id = EntityId::from_bytes([3; 16]);
    {
        let store = SqliteSessionStore::open(&path).expect("open store");
        let mut root = Branch::new(root_id, "root").expect("root");
        root.model_invocations.push(ModelInvocation {
            id: EntityId::from_bytes([4; 16]),
            branch: root_id,
            ordinal: 1,
            agent_profile: None,
            context_fingerprint: Some("context-fingerprint".to_owned()),
            provider_profile: Some("provider-a".to_owned()),
            provider_id: None,
            model: Some("model-a".to_owned()),
            model_profile: Some("profile-a".to_owned()),
            provider_model_id: Some("native-model-a".to_owned()),
            thinking_level: Some("careful".to_owned()),
            request_fingerprint: Some("request-fingerprint".to_owned()),
            cache: CacheDisposition::NotUsed,
            output: None,
            request: None,
        });
        let mut session = Session {
            id: session_id,
            branches: vec![root],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        };
        session
            .add_child_branch(child_id, root_id, "child", "inspect")
            .expect("child");
        session
            .execution
            .replace(
                0,
                Some("provider-a".to_owned()),
                Some("model-a".to_owned()),
                Some("careful".to_owned()),
            )
            .expect("execution selection");
        store.save(&session).expect("save session");
    }
    let reopened = SqliteSessionStore::open(&path).expect("reopen store");
    let loaded = reopened
        .load(session_id)
        .expect("load session")
        .expect("stored session");
    assert_eq!(loaded.id, session_id);
    assert_eq!(loaded.branches.len(), 2);
    assert_eq!(loaded.branches[0].children, vec![child_id]);
    assert_eq!(loaded.execution.revision, 1);
    assert_eq!(loaded.execution.provider_id.as_deref(), Some("provider-a"));
    assert_eq!(loaded.execution.model_id.as_deref(), Some("model-a"));
    assert_eq!(loaded.execution.thinking_level.as_deref(), Some("careful"));
    assert_eq!(
        loaded.branches[0].model_invocations[0]
            .provider_model_id
            .as_deref(),
        Some("native-model-a")
    );
    assert_eq!(
        loaded.branches[0].model_invocations[0]
            .thinking_level
            .as_deref(),
        Some("careful")
    );
    let _ = std::fs::remove_file(path);
}

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
        root_nodes: vec![first, second, downstream],
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

#[path = "durable_parts/part1.rs"]
mod part1;
#[path = "durable_parts/request_provenance.rs"]
mod request_provenance;

#[path = "durable_parts/busy_timeout.rs"]
mod busy_timeout;
#[path = "durable_parts/claim_revision_scope.rs"]
mod claim_revision_scope;
#[path = "durable_parts/incremental_rollup.rs"]
mod incremental_rollup;
#[path = "durable_parts/lifecycle_revisions.rs"]
mod lifecycle_revisions;
#[path = "durable_parts/part2.rs"]
mod part2;
#[path = "durable_parts/plan_history_migration.rs"]
mod plan_history_migration;
#[path = "durable_parts/plan_projection.rs"]
mod plan_projection;
#[path = "durable_parts/rollup_index.rs"]
mod rollup_index;
