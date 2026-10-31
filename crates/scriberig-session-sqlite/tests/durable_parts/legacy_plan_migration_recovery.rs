//! Verifies interrupted legacy plan imports resume stale-claim recovery.

use rusqlite::{Connection, params};
use scriberig_core::EntityId;
use scriberig_session::{
    Branch, ContinuationAnchor, Plan, PlanNode, PlanNodeParent, PlanNodeStatus, PlanStatus,
    Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;

/// Builds a legacy plan containing active coordinator and worker claims.
///
/// # Returns
/// The session, plan identity, worker branch, and pre-recovery revision.
fn inflight_legacy_session() -> (Session, EntityId, EntityId, u64) {
    let session_id = EntityId::from_bytes([121; 16]);
    let root_branch = EntityId::from_bytes([122; 16]);
    let worker_branch = EntityId::from_bytes([123; 16]);
    let plan_id = EntityId::from_bytes([124; 16]);
    let node_id = EntityId::from_bytes([125; 16]);
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
        "resume interrupted migration",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(PlanNodeParent::Branch(root_branch)),
        title: "in-flight work".to_owned(),
        purpose: "recover its stale claim after migration".to_owned(),
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
    .expect("node");
    plan.transition(PlanStatus::Planned).expect("plan ready");
    plan.start_with_coordinator_claim(EntityId::from_bytes([126; 16]), "old-coordinator", None)
        .expect("coordinator claim");
    plan.start_node(
        node_id,
        worker_branch,
        EntityId::from_bytes([127; 16]),
        "old-worker",
        &[],
    )
    .expect("worker claim");
    let revision = plan.revision;
    (
        Session {
            id: session_id,
            branches: vec![
                Branch::new(root_branch, "root").expect("root branch"),
                Branch::new(worker_branch, "worker").expect("worker branch"),
            ],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        },
        plan_id,
        worker_branch,
        revision,
    )
}

/// Reads whether a legacy plan's recovery marker remains stored.
///
/// # Parameters
/// - `connection`: Database containing the session and semantic stores.
/// - `session`: Owning session identity.
/// - `plan`: Stable plan identity.
///
/// # Returns
/// Whether claim recovery remains pending.
fn migration_recovery_pending(connection: &Connection, session: EntityId, plan: EntityId) -> bool {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM plan_legacy_migration_pending
             WHERE session_id=?1 AND plan_id=?2)",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .expect("read migration marker")
}

/// Resumes pending claim recovery after a partial semantic migration and reopen.
#[test]
fn interrupted_legacy_plan_migration_resumes_claim_recovery_after_reopen() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-legacy-plan-recovery-{}.sqlite",
        EntityId::new()
    ));
    let (session, plan_id, worker_branch, original_revision) = inflight_legacy_session();
    let session_id = session.id;
    let store = SqliteSessionStore::open(&path).expect("open store");
    Connection::open(&path)
        .expect("open legacy fixture connection")
        .execute(
            "INSERT INTO session_records(session_id, payload) VALUES (?1, ?2)",
            params![
                session_id.as_bytes().as_slice(),
                serde_json::to_vec(&session).expect("encode legacy session")
            ],
        )
        .expect("insert legacy session");
    Connection::open(&path)
        .expect("open failure injection connection")
        .execute_batch(
            "CREATE TRIGGER interrupt_plan_migration
             BEFORE UPDATE ON roots
             BEGIN SELECT RAISE(ABORT, 'injected migration interruption'); END;",
        )
        .expect("install migration interruption");

    assert!(store.load(session_id).is_err());
    let connection = Connection::open(&path).expect("inspect partial migration");
    let root_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM roots WHERE name=?1)",
            params![format!("scriberig-plan-{session_id}-{plan_id}")],
            |row| row.get(0),
        )
        .expect("check imported history prefix");
    assert!(root_exists, "a prefix of the semantic history was imported");
    assert!(migration_recovery_pending(&connection, session_id, plan_id));
    drop(connection);
    drop(store);

    Connection::open(&path)
        .expect("open trigger cleanup connection")
        .execute_batch("DROP TRIGGER interrupt_plan_migration")
        .expect("remove injected failure");
    let reopened = SqliteSessionStore::open(&path).expect("reopen store");
    let migrated = reopened
        .load(session_id)
        .expect("resume migration")
        .expect("session remains stored");
    let migrated_plan = &migrated.plans[0];
    let migrated_node = &migrated_plan.nodes[0];
    assert_eq!(migrated_plan.revision, original_revision + 1);
    assert!(migrated_plan.coordinator_claim.is_none());
    assert_eq!(migrated_node.status, PlanNodeStatus::Failed);
    assert!(migrated_node.claim.is_none());
    assert_eq!(migrated_node.branch, Some(worker_branch));
    assert!(migrated_plan.history.iter().any(|revision| {
        revision.revision == original_revision
            && revision.nodes.iter().any(|node| {
                node.id == EntityId::from_bytes([125; 16])
                    && node.status == PlanNodeStatus::Running
                    && node.claim.is_some()
            })
    }));
    let connection = Connection::open(&path).expect("inspect recovered migration");
    assert!(!migration_recovery_pending(
        &connection,
        session_id,
        plan_id
    ));
    drop(connection);
    drop(reopened);
    let _ = std::fs::remove_file(path);
}
