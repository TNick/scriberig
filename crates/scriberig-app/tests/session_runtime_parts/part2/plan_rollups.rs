//! Verifies application plan-rollup reads use valid caches and canonical fallback.

use super::*;
use scriberig_session::{
    Branch, Plan, PlanNode, PlanNodeParent, PlanNodeStatus, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;
use std::sync::Arc;

#[test]
/// Reads cached rollups and falls back after derived rows are removed.
fn application_reads_snapshot_rollups_from_cache_and_canonical_state() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-app-plan-rollups-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([201; 16]);
    let branch_id = EntityId::from_bytes([202; 16]);
    let plan_id = EntityId::from_bytes([203; 16]);
    let parent_id = EntityId::from_bytes([204; 16]);
    let failed_id = EntityId::from_bytes([205; 16]);
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "read cached plan rollups",
    )
    .expect("plan");
    plan.add_node(node(
        parent_id,
        PlanNodeParent::Branch(branch_id),
        PlanNodeStatus::Planned,
    ))
    .expect("parent node");
    plan.add_node(node(
        failed_id,
        PlanNodeParent::Node(parent_id),
        PlanNodeStatus::Failed,
    ))
    .expect("failed child");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan lifecycle");
    let store = Arc::new(SqliteSessionStore::open(&path).expect("store"));
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(branch_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("persist plan");
    let stored = store
        .load(session_id)
        .expect("load session")
        .expect("session");
    let stored_plan = &stored.plans[0];
    let snapshot = stored_plan.source_snapshot.expect("plan snapshot");
    let reference = stored_plan.node_rollups().expect("reference evaluation");
    let service = ApplicationService::new(Fixture).with_session_store(store.clone());

    let cached = service
        .read_plan_rollups(&session_context(), session_id, plan_id, snapshot)
        .expect("read cached rollups");
    assert_eq!(cached, reference);
    let failed_children = stored_plan
        .nodes
        .iter()
        .filter(|node| node.parent == Some(PlanNodeParent::Node(parent_id)))
        .filter(|node| node.status == PlanNodeStatus::Failed)
        .count();
    assert_eq!(
        cached[&parent_id].child_counts.failed,
        u32::try_from(failed_children).expect("bounded fixture count")
    );

    let connection = rusqlite::Connection::open(&path).expect("open cache database");
    connection
        .execute(
            "DELETE FROM plan_node_rollup_index WHERE session_id=?1 AND plan_id=?2",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice()
            ],
        )
        .expect("remove derived cache");
    drop(connection);
    let rebuilt = service
        .read_plan_rollups(&session_context(), session_id, plan_id, snapshot)
        .expect("fall back to canonical evaluation");
    assert_eq!(rebuilt, reference);
    drop(service);
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Creates one plan node for the rollup application test.
///
/// # Parameters
/// - `id`: Stable node identity.
/// - `parent`: Branch or plan-node structural parent.
/// - `status`: Canonical lifecycle status for the node.
///
/// # Returns
/// The requested plan node with no assignment or result.
fn node(id: EntityId, parent: PlanNodeParent, status: PlanNodeStatus) -> PlanNode {
    PlanNode {
        id,
        parent: Some(parent),
        title: "work".to_owned(),
        purpose: "test rollup reads".to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status,
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
