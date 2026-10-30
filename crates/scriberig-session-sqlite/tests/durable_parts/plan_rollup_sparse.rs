//! Verifies plan CAS updates only changed SQLite direct and rollup rows.

use rusqlite::{Connection, params};
use scriberig_core::EntityId;
use scriberig_session::{
    Branch, ContinuationAnchor, Plan, PlanGate, PlanGatePhase, PlanGateRequirement, PlanNode,
    PlanNodeParent, PlanNodeRollupIndex, PlanNodeStatus, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;
use std::fmt::Write as _;
use std::path::Path;

/// Creates one directly scheduled plan node.
///
/// # Parameters
/// - `id`: Stable node identity.
/// - `parent`: Structural parent identity.
/// - `title`: Human-readable work title.
///
/// # Returns
/// A planned node without assignment or active claim.
fn planned_node(id: EntityId, parent: PlanNodeParent, title: &str) -> PlanNode {
    PlanNode {
        id,
        parent: Some(parent),
        title: title.to_owned(),
        purpose: title.to_owned(),
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

/// Creates and persists the plan used by the sparse-write regression.
///
/// # Parameters
/// - `path`: SQLite database path for the test store.
///
/// # Returns
/// The store, owning session and plan identities, and affected/unrelated nodes.
fn sparse_rollup_fixture(
    path: &Path,
) -> (
    SqliteSessionStore,
    EntityId,
    EntityId,
    EntityId,
    EntityId,
    EntityId,
) {
    let session_id = EntityId::from_bytes([131; 16]);
    let branch_id = EntityId::from_bytes([132; 16]);
    let plan_id = EntityId::from_bytes([133; 16]);
    let parent_id = EntityId::from_bytes([134; 16]);
    let leaf_id = EntityId::from_bytes([135; 16]);
    let unrelated_id = EntityId::from_bytes([136; 16]);
    let store = SqliteSessionStore::open(path).expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        ContinuationAnchor::BranchHead { branch: branch_id },
        "sparse SQLite rollups",
    )
    .expect("plan");
    plan.add_node(planned_node(
        parent_id,
        PlanNodeParent::Branch(branch_id),
        "ancestor",
    ))
    .expect("parent node");
    plan.add_node(planned_node(
        leaf_id,
        PlanNodeParent::Node(parent_id),
        "changed leaf",
    ))
    .expect("leaf node");
    plan.add_node(planned_node(
        unrelated_id,
        PlanNodeParent::Branch(branch_id),
        "unrelated root",
    ))
    .expect("unrelated node");
    plan.add_gate(PlanGate {
        id: EntityId::from_bytes([137; 16]),
        owner: parent_id,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::RequiredChildCompleted { child: leaf_id },
        waiver: None,
    })
    .expect("required child gate");
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(branch_id, "root").expect("branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save plan");
    (store, session_id, plan_id, parent_id, leaf_id, unrelated_id)
}

/// Installs triggers that reject writes to an unrelated direct or rollup row.
///
/// # Parameters
/// - `path`: SQLite database path for the test store.
/// - `unrelated`: Stable identity whose cache row must remain untouched.
fn reject_unrelated_rollup_writes(path: &Path, unrelated: EntityId) {
    let connection = Connection::open(path).expect("open trigger connection");
    let mut unrelated_hex = String::with_capacity(32);
    for byte in unrelated.as_bytes() {
        write!(unrelated_hex, "{byte:02x}").expect("write identity hex");
    }
    connection
        .execute_batch(&format!(
            "CREATE TRIGGER reject_unrelated_rollup_update
             BEFORE UPDATE ON plan_node_rollup_index
             WHEN OLD.node_id=X'{unrelated_hex}'
             BEGIN SELECT RAISE(ABORT, 'unrelated rollup row was updated'); END;
             CREATE TRIGGER reject_unrelated_rollup_delete
             BEFORE DELETE ON plan_node_rollup_index
             WHEN OLD.node_id=X'{unrelated_hex}'
             BEGIN SELECT RAISE(ABORT, 'unrelated rollup row was deleted'); END;
             CREATE TRIGGER reject_unrelated_node_update
             BEFORE UPDATE ON plan_node_index
             WHEN OLD.node_id=X'{unrelated_hex}'
             BEGIN SELECT RAISE(ABORT, 'unrelated direct row was updated'); END;
             CREATE TRIGGER reject_unrelated_node_delete
             BEFORE DELETE ON plan_node_index
             WHEN OLD.node_id=X'{unrelated_hex}'
             BEGIN SELECT RAISE(ABORT, 'unrelated direct row was deleted'); END;"
        ))
        .expect("install sparse-write assertions");
}

/// Asserts that plan-level cache metadata identifies the exact successor.
///
/// # Parameters
/// - `path`: SQLite database path for the test store.
/// - `session`: Owning session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact successor snapshot represented by the cache.
fn assert_cache_metadata(path: &Path, session: EntityId, plan: EntityId, snapshot: &[u8]) {
    let connection = Connection::open(path).expect("inspect metadata");
    let metadata: (Vec<u8>, i64, i64) = connection
        .query_row(
            "SELECT source_snapshot, evaluator_version, node_count
             FROM plan_node_rollup_meta WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read cache metadata");
    assert_eq!(metadata.0.as_slice(), snapshot);
    assert_eq!(metadata.1, 1);
    assert_eq!(metadata.2, 3);
    let direct: (Vec<u8>, i64) = connection
        .query_row(
            "SELECT source_snapshot, node_count FROM plan_node_index_meta
             WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read direct index metadata");
    assert_eq!(direct.0.as_slice(), snapshot);
    assert_eq!(direct.1, 3);
}

/// Prevents CAS from deleting or rewriting unrelated direct and rollup rows.
#[test]
fn plan_cas_keeps_unrelated_sqlite_rollup_rows_untouched() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-plan-rollup-sparse-{}.sqlite",
        EntityId::new()
    ));
    let (store, session_id, plan_id, parent_id, leaf_id, unrelated_id) =
        sparse_rollup_fixture(&path);
    let before = store
        .load(session_id)
        .expect("load plan")
        .expect("stored session");
    let old_snapshot = before.plans[0].source_snapshot.expect("base snapshot");
    let cached = store
        .load_plan_rollups_at_snapshot(session_id, plan_id, old_snapshot)
        .expect("load base rollups")
        .expect("complete base cache");
    let mut proposed = before.clone();
    proposed.plans[0]
        .nodes
        .iter_mut()
        .find(|node| node.id == leaf_id)
        .expect("leaf")
        .status = PlanNodeStatus::Failed;
    proposed.plans[0].revision += 1;
    let unrelated_before = cached[&unrelated_id];
    reject_unrelated_rollup_writes(&path, unrelated_id);

    store
        .compare_and_swap_plan(&proposed, plan_id, before.plans[0].revision)
        .expect("promote leaf failure with sparse rollup writes");
    let after = store
        .load(session_id)
        .expect("reload plan")
        .expect("stored session");
    let new_snapshot = after.plans[0].source_snapshot.expect("successor snapshot");
    let rollups = store
        .load_plan_rollups_at_snapshot(session_id, plan_id, new_snapshot)
        .expect("read current rollups")
        .expect("validated current cache");
    let expected =
        PlanNodeRollupIndex::rebuild(&after.plans[0], new_snapshot).expect("reference rollups");
    assert_ne!(old_snapshot, new_snapshot);
    assert_eq!(rollups, *expected.all());
    assert_eq!(rollups[&leaf_id].direct_status, PlanNodeStatus::Failed);
    assert_eq!(rollups[&parent_id].effective_status, PlanNodeStatus::Failed);
    assert_eq!(rollups[&unrelated_id], unrelated_before);
    assert_cache_metadata(&path, session_id, plan_id, new_snapshot.as_bytes());
    drop(store);
    let _ = std::fs::remove_file(path);
}
