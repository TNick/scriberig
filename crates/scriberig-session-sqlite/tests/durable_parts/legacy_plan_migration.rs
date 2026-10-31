//! Verifies multi-revision legacy plans migrate without losing results.

use rusqlite::{Connection, params};
use scriberig_core::EntityId;
use scriberig_session::{
    Branch, BranchResult, BranchStatus, ContinuationAnchor, ModelAssignment, Plan, PlanNode,
    PlanNodeParent, PlanNodeStatus, PlanStatus, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The regression checks a full multi-revision legacy import."
)]
/// Imports multiple aggregate revisions while preserving terminal node data.
fn legacy_plan_migration_preserves_historical_completed_result() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-legacy-plan-history-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([81; 16]);
    let root_branch = EntityId::from_bytes([82; 16]);
    let plan_id = EntityId::from_bytes([83; 16]);
    let node_id = EntityId::from_bytes([84; 16]);
    let dependent_id = EntityId::from_bytes([90; 16]);
    let superseded_id = EntityId::from_bytes([91; 16]);
    let replacement_id = EntityId::from_bytes([92; 16]);
    let worker_branch = EntityId::from_bytes([85; 16]);
    let claim_id = EntityId::from_bytes([86; 16]);
    let result_id = EntityId::from_bytes([87; 16]);
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
        "migrate historical plan",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(root_branch)),
        title: "complete work".to_owned(),
        purpose: "retain the exact result in history".to_owned(),
        expected_result: None,
        semantic_scope: vec!["semantic:parser".to_owned()],
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: Some(ModelAssignment {
            provider_id: "provider-a".to_owned(),
            model_id: "model-a".to_owned(),
            thinking_level: Some("high".to_owned()),
        }),
        claim: None,
        claim_generation: 0,
    })
    .expect("node");
    plan.add_node(PlanNode {
        id: dependent_id,
        parent: Some(PlanNodeParent::Node(node_id)),
        title: "consume result".to_owned(),
        purpose: "preserve topology and dependency".to_owned(),
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
    .expect("dependent node");
    plan.add_node(PlanNode {
        id: superseded_id,
        parent: Some(PlanNodeParent::Branch(root_branch)),
        title: "obsolete work".to_owned(),
        purpose: "be replaced in the paused plan".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: Some(ModelAssignment {
            provider_id: "provider-legacy".to_owned(),
            model_id: "model-legacy".to_owned(),
            thinking_level: Some("high".to_owned()),
        }),
        claim: None,
        claim_generation: 0,
    })
    .expect("superseded node");
    plan.add_dependency(node_id, dependent_id)
        .expect("dependency");
    plan.transition(PlanStatus::Planned).expect("plan");
    plan.start_with_coordinator_claim(EntityId::from_bytes([89; 16]), "coordinator", None)
        .expect("start plan");
    plan.start_node(node_id, worker_branch, claim_id, "worker", &[])
        .expect("claim node");
    let result = BranchResult {
        id: result_id,
        branch: worker_branch,
        status: BranchStatus::Completed,
        summary: Some("parser completed".to_owned()),
        result_kind: Some("semantic_changes".to_owned()),
        candidate_context: None,
        references: vec![EntityId::from_bytes([88; 16])],
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    plan.complete_node(node_id, claim_id, &result)
        .expect("complete work");
    plan.transition(PlanStatus::Paused).expect("pause plan");
    plan.replan(
        vec![PlanNode {
            id: replacement_id,
            parent: Some(PlanNodeParent::Branch(root_branch)),
            title: "replacement work".to_owned(),
            purpose: "replace obsolete work".to_owned(),
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
        Vec::new(),
        &[],
        &[(superseded_id, replacement_id)],
        &[],
        &[node_id],
        "replace obsolete work",
    )
    .expect("replace stale planned node");
    let expected_history = plan.history.clone();
    let expected_plan = plan.clone();
    let expected_coordinator_generation = plan.coordinator_generation;
    let session = Session {
        id: session_id,
        branches: vec![
            Branch::new(root_branch, "root").expect("root branch"),
            Branch::new(worker_branch, "worker").expect("worker branch"),
        ],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let raw_payload = serde_json::to_value(&session).expect("legacy aggregate payload");
    let store = SqliteSessionStore::open(&path).expect("open store");
    insert_legacy_payload(&path, session_id, &raw_payload);

    let loaded = store
        .load(session_id)
        .expect("migrate legacy session")
        .expect("session exists");
    let migrated = &loaded.plans[0];

    let connection = Connection::open(&path).expect("inspect migrated compatibility payload");
    let stored_payload = connection
        .query_row(
            "SELECT payload FROM session_records WHERE session_id=?1",
            params![session_id.as_bytes().as_slice()],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .expect("read migrated payload");
    let stored_value: serde_json::Value =
        serde_json::from_slice(&stored_payload).expect("decode migrated payload");
    assert_eq!(stored_value["plans"], serde_json::json!([]));
    drop(connection);

    assert_eq!(migrated.id, plan_id);
    assert_eq!(migrated.revision, expected_plan.revision);
    assert_eq!(migrated.goal, expected_plan.goal);
    assert_eq!(migrated.status, expected_plan.status);
    assert_eq!(migrated.coordinator, expected_plan.coordinator);
    assert_eq!(
        migrated.coordinator_cursor,
        expected_plan.coordinator_cursor
    );
    assert_eq!(migrated.nodes[0].id, node_id);
    assert_eq!(migrated.nodes[0].status, PlanNodeStatus::Completed);
    assert_eq!(migrated.nodes[0].result, Some(result_id));
    assert_eq!(migrated.nodes[0].branch, Some(worker_branch));
    assert_eq!(
        migrated.nodes[0].assignment,
        expected_plan.nodes[0].assignment
    );
    assert_eq!(migrated.nodes[1].id, dependent_id);
    assert_eq!(
        migrated.nodes[1].parent,
        Some(PlanNodeParent::Node(node_id))
    );
    assert_eq!(migrated.root_nodes, expected_plan.root_nodes);
    assert_eq!(migrated.dependencies.len(), 1);
    assert_eq!(migrated.dependencies, expected_plan.dependencies);
    assert_eq!(migrated.nodes[2].id, superseded_id);
    assert_eq!(migrated.nodes[2].superseded_by, Some(replacement_id));
    assert_eq!(migrated.nodes[3].id, replacement_id);
    assert_eq!(migrated.nodes, expected_plan.nodes);
    assert_eq!(
        migrated.coordinator_generation,
        expected_coordinator_generation
    );
    let completed_revision = migrated
        .history
        .iter()
        .find(|revision| {
            revision.status == PlanStatus::Running
                && revision.nodes.iter().any(|node| {
                    node.id == node_id
                        && node.status == PlanNodeStatus::Completed
                        && node.result == Some(result_id)
                })
        })
        .expect("historical completion with exact result");
    let expected_completed_revision = expected_history
        .iter()
        .find(|revision| {
            revision.status == PlanStatus::Running
                && revision.nodes.iter().any(|node| {
                    node.id == node_id
                        && node.status == PlanNodeStatus::Completed
                        && node.result == Some(result_id)
                })
        })
        .expect("legacy completed revision");
    assert_eq!(
        completed_revision.revision,
        expected_completed_revision.revision
    );
    assert!(migrated.history.len() >= 5);

    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Inserts a raw unversioned session payload as an older repository would.
///
/// # Parameters
/// - `path`: Initialized SQLite session database path.
/// - `session`: Stable session identity for the row key.
/// - `payload`: Legacy aggregate JSON payload.
fn insert_legacy_payload(path: &std::path::Path, session: EntityId, payload: &serde_json::Value) {
    let connection = Connection::open(path).expect("open legacy insert connection");
    connection
        .execute(
            "INSERT INTO session_records(session_id, payload) VALUES (?1, ?2)",
            params![
                session.as_bytes().as_slice(),
                serde_json::to_vec(payload).expect("encode legacy session")
            ],
        )
        .expect("insert legacy session");
}
