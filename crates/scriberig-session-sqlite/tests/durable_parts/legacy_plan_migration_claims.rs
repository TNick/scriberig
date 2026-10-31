//! Verifies legacy runtime metadata survives safe stale-claim recovery.

use rusqlite::Connection;
use scriberig_core::{EntityId, OperationId, RootName};
use scriberig_session::{
    Branch, ContinuationAnchor, ModelAssignment, Plan, PlanNode, PlanNodeParent, PlanNodeStatus,
    PlanStatus, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store::{RootStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Recovers legacy runtime claims instead of restoring them as active work.
#[test]
fn legacy_plan_migration_recovers_stale_worker_and_coordinator_claims() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-legacy-plan-claims-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([101; 16]);
    let root_branch = EntityId::from_bytes([102; 16]);
    let plan_id = EntityId::from_bytes([103; 16]);
    let node_id = EntityId::from_bytes([104; 16]);
    let worker_branch = EntityId::from_bytes([105; 16]);
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
        "recover imported claims",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(root_branch)),
        title: "in-flight work".to_owned(),
        purpose: "must not be blindly resumed".to_owned(),
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
    .expect("node");
    plan.set_coordinator(root_branch, Some(node_id))
        .expect("legacy coordinator cursor");
    plan.transition(PlanStatus::Planned).expect("plan ready");
    let coordinator_operation =
        OperationId::new("legacy-coordinator-operation").expect("coordinator operation");
    plan.start_with_coordinator_claim(
        EntityId::from_bytes([106; 16]),
        "old-coordinator",
        Some(coordinator_operation.clone()),
    )
    .expect("coordinator claim");
    plan.start_node(
        node_id,
        worker_branch,
        EntityId::from_bytes([107; 16]),
        "old-worker",
        &[],
    )
    .expect("worker claim");
    let original_revision = plan.revision;
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
    assert_recovered_claims(
        migrated,
        original_revision,
        root_branch,
        worker_branch,
        node_id,
    );
    assert_imported_claim_history(
        &store,
        &path,
        session_id,
        plan_id,
        original_revision,
        &coordinator_operation,
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Checks recovered worker and coordinator metadata in the current projection.
///
/// # Parameters
/// - `migrated`: Recovered canonical plan.
/// - `original_revision`: Revision of the imported active-claim state.
/// - `root_branch`: Original coordinator branch.
/// - `worker_branch`: Original claimed worker branch.
/// - `node_id`: Stable claimed node identity.
fn assert_recovered_claims(
    migrated: &Plan,
    original_revision: u64,
    root_branch: EntityId,
    worker_branch: EntityId,
    node_id: EntityId,
) {
    let node = &migrated.nodes[0];
    assert_eq!(migrated.revision, original_revision + 1);
    assert!(migrated.coordinator_claim.is_none());
    assert_eq!(node.status, PlanNodeStatus::Failed);
    assert!(node.claim.is_none());
    assert_eq!(node.branch, Some(worker_branch));
    assert_eq!(node.claim_generation, 1);
    assert_eq!(
        node.assignment
            .as_ref()
            .map(|value| value.model_id.as_str()),
        Some("model-legacy")
    );
    assert_eq!(migrated.coordinator, Some(root_branch));
    assert_eq!(migrated.coordinator_cursor, Some(node_id));
    assert_eq!(migrated.coordinator_generation, 1);
    let attempts = migrated
        .attempt_history(node_id)
        .expect("known migrated node attempts");
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].branch, worker_branch);
    assert_eq!(attempts[0].claim_generation, 1);
    assert_eq!(attempts[0].claim, Some(EntityId::from_bytes([107; 16])));
    assert_eq!(attempts[0].status, PlanNodeStatus::Failed);
    assert!(migrated.history.iter().any(|revision| {
        revision.revision == original_revision
            && revision.nodes.iter().any(|historical| {
                historical.status == PlanNodeStatus::Running
                    && historical.claim.as_ref().is_some_and(|claim| {
                        claim.owner == "old-worker"
                            && claim
                                .assignment
                                .as_ref()
                                .is_some_and(|assignment| assignment.model_id == "model-legacy")
                    })
            })
    }));
}

/// Checks the imported snapshot and cleared durable migration marker.
///
/// # Parameters
/// - `store`: Session store used to resolve historical plan state.
/// - `path`: SQLite database path.
/// - `session_id`: Stable session identity.
/// - `plan_id`: Stable plan identity.
/// - `original_revision`: Revision containing the imported active claims.
/// - `coordinator_operation`: Legacy coordinator operation identity.
fn assert_imported_claim_history(
    store: &SqliteSessionStore,
    path: &std::path::Path,
    session_id: EntityId,
    plan_id: EntityId,
    original_revision: u64,
    coordinator_operation: &OperationId,
) {
    let semantic_store = SqliteStore::open(path).expect("open semantic store");
    let root = RootName::new(format!("scriberig-plan-{session_id}-{plan_id}")).expect("plan root");
    let current_snapshot = semantic_store.read_root(&root).expect("read plan root");
    let imported_claim_snapshot = semantic_store
        .ancestors(current_snapshot)
        .expect("read plan ancestry")
        .into_iter()
        .find_map(|snapshot| {
            let contains_plan = semantic_store
                .iter(snapshot)
                .expect("read ancestry snapshot")
                .iter()
                .any(|(entity, _)| *entity == plan_id);
            if !contains_plan {
                return None;
            }
            store
                .load_plan_at_snapshot(session_id, plan_id, snapshot)
                .expect("load imported plan snapshot")
                .filter(|projection| projection.revision == original_revision)
        })
        .expect("original imported claim snapshot");
    let historical_coordinator = imported_claim_snapshot
        .coordinator_claim
        .expect("historical coordinator claim");
    assert_eq!(historical_coordinator.id, EntityId::from_bytes([106; 16]));
    assert_eq!(historical_coordinator.owner, "old-coordinator");
    assert_eq!(
        historical_coordinator.operation,
        Some(coordinator_operation.clone())
    );
    assert_eq!(historical_coordinator.generation, 1);
    let connection = Connection::open(path).expect("open marker assertion connection");
    let recovery_pending: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM plan_legacy_migration_pending
             WHERE session_id=?1 AND plan_id=?2)",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice()
            ],
            |row| row.get(0),
        )
        .expect("read migration marker");
    assert!(!recovery_pending);
}

/// Inserts a raw legacy aggregate payload into an initialized session database.
///
/// # Parameters
/// - `path`: Initialized SQLite session database path.
/// - `session`: Stable session identity for the row key.
/// - `payload`: Legacy aggregate JSON payload.
fn insert_legacy_payload(path: &std::path::Path, session: EntityId, payload: &serde_json::Value) {
    Connection::open(path)
        .expect("open legacy insert connection")
        .execute(
            "INSERT INTO session_records(session_id, payload) VALUES (?1, ?2)",
            rusqlite::params![
                session.as_bytes().as_slice(),
                serde_json::to_vec(payload).expect("encode legacy session")
            ],
        )
        .expect("insert legacy session");
}
