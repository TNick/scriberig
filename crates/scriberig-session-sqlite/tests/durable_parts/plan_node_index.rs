//! Verifies the normalized SQLite plan-node index and its rebuild helper.

use super::*;
use rusqlite::Connection;
use scriberig_session::{PlanNodeParent, PlanNodeStatus, PlanSemanticSnapshot};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "One durable lifecycle verifies index creation, CAS refresh, and rebuild."
)]
/// Indexes node revisions and parents, then rebuilds missing index rows.
fn sqlite_plan_node_index_tracks_snapshot_and_rebuilds() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-plan-node-index-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([91; 16]);
    let branch_id = EntityId::from_bytes([92; 16]);
    let plan_id = EntityId::from_bytes([93; 16]);
    let parent_id = EntityId::from_bytes([94; 16]);
    let child_id = EntityId::from_bytes([95; 16]);
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "index plan nodes",
    )
    .expect("plan");
    plan.add_node(node(parent_id, PlanNodeParent::Branch(branch_id)))
        .expect("parent node");
    plan.add_node(node(child_id, PlanNodeParent::Node(parent_id)))
        .expect("child node");
    let store = SqliteSessionStore::open(&path).expect("open store");
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(branch_id, "root").expect("branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan.clone()],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save plan");

    let stored_plan = store
        .load(session_id)
        .expect("load plan")
        .expect("session")
        .plans[0]
        .clone();
    let semantic_snapshot = PlanSemanticSnapshot::from_plan(&stored_plan).expect("encode plan");
    let node_refs = semantic_snapshot.node_refs().expect("plan node refs");
    let connection = Connection::open(&path).expect("open query connection");
    let indexed: (Vec<u8>, String, Vec<u8>, Vec<u8>) = connection
        .query_row(
            "SELECT parent_node_id, direct_status, current_revision_id, source_snapshot
             FROM plan_node_index WHERE session_id=?1 AND plan_id=?2 AND node_id=?3",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice(),
                child_id.as_bytes().as_slice(),
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("read indexed child");
    let child_ref = node_refs
        .iter()
        .find(|node| node.entity == child_id)
        .expect("child reference");
    assert_eq!(indexed.0, parent_id.as_bytes());
    assert_eq!(indexed.1, "ready");
    assert_eq!(indexed.2, child_ref.revision.as_bytes());
    assert_eq!(
        indexed.3,
        stored_plan
            .source_snapshot
            .expect("source snapshot")
            .as_bytes()
    );
    assert_eq!(child_ref.status, PlanNodeStatus::Ready);
    let expected_rollup = stored_plan.node_rollups().expect("reference rollups")[&parent_id];
    let indexed_rollup: (String, i64, i64, [i64; 7], Vec<u8>, i64) = connection
        .query_row(
            "SELECT effective_status, contains_ignored, contains_abandoned,
                    child_planned, child_ready, child_running, child_completed,
                    child_failed, child_abandoned, unsatisfied_completion_gates,
                    source_snapshot, evaluator_version
             FROM plan_node_rollup_index
             WHERE session_id=?1 AND plan_id=?2 AND node_id=?3",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice(),
                parent_id.as_bytes().as_slice(),
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    [
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                    ],
                    row.get(10)?,
                    row.get(11)?,
                ))
            },
        )
        .expect("read indexed rollup");
    assert_eq!(
        indexed_rollup.0,
        status_tag(expected_rollup.effective_status)
    );
    assert_eq!(indexed_rollup.1 != 0, expected_rollup.contains_ignored);
    assert_eq!(indexed_rollup.2 != 0, expected_rollup.contains_abandoned);
    assert_eq!(
        indexed_rollup.3,
        [
            i64::from(expected_rollup.child_counts.planned),
            i64::from(expected_rollup.child_counts.ready),
            i64::from(expected_rollup.child_counts.running),
            i64::from(expected_rollup.child_counts.completed),
            i64::from(expected_rollup.child_counts.failed),
            i64::from(expected_rollup.child_counts.abandoned),
            i64::from(expected_rollup.unsatisfied_completion_gates),
        ]
    );
    assert_eq!(
        indexed_rollup.4,
        stored_plan
            .source_snapshot
            .expect("source snapshot")
            .as_bytes()
    );
    assert_eq!(indexed_rollup.5, 1);

    let mut changed_session = store
        .load(session_id)
        .expect("load plan before mutation")
        .expect("session");
    let expected_revision = changed_session.plans[0].revision;
    changed_session.plans[0]
        .update_summary(scriberig_session::PlanNodeSummary {
            id: EntityId::from_bytes([96; 16]),
            node: child_id,
            text: "node index source moved".to_owned(),
            sources: Vec::new(),
        })
        .expect("update summary");
    store
        .compare_and_swap_plan(&changed_session, plan_id, expected_revision)
        .expect("persist changed snapshot");
    let updated_snapshot = store
        .load(session_id)
        .expect("load updated plan")
        .expect("session")
        .plans[0]
        .source_snapshot
        .expect("updated source snapshot");
    let indexed_snapshot: Vec<u8> = connection
        .query_row(
            "SELECT source_snapshot FROM plan_node_index
             WHERE session_id=?1 AND plan_id=?2 AND node_id=?3",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice(),
                child_id.as_bytes().as_slice(),
            ],
            |row| row.get(0),
        )
        .expect("read updated index snapshot");
    assert_eq!(indexed_snapshot.as_slice(), updated_snapshot.as_bytes());
    let rollup_snapshot: Vec<u8> = connection
        .query_row(
            "SELECT source_snapshot FROM plan_node_rollup_index
             WHERE session_id=?1 AND plan_id=?2 AND node_id=?3",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice(),
                child_id.as_bytes().as_slice(),
            ],
            |row| row.get(0),
        )
        .expect("read updated rollup snapshot");
    assert_eq!(rollup_snapshot.as_slice(), updated_snapshot.as_bytes());

    connection
        .execute(
            "DELETE FROM plan_node_index WHERE session_id=?1 AND plan_id=?2",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice()
            ],
        )
        .expect("delete index rows");
    connection
        .execute(
            "DELETE FROM plan_node_rollup_index WHERE session_id=?1 AND plan_id=?2",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice()
            ],
        )
        .expect("delete rollup rows");
    store
        .rebuild_plan_node_index(session_id, plan_id)
        .expect("rebuild index");
    let indexed_count: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM plan_node_index WHERE session_id=?1 AND plan_id=?2",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice()
            ],
            |row| row.get(0),
        )
        .expect("count rebuilt rows");
    assert_eq!(indexed_count, 2);
    let rollup_count: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM plan_node_rollup_index WHERE session_id=?1 AND plan_id=?2",
            rusqlite::params![
                session_id.as_bytes().as_slice(),
                plan_id.as_bytes().as_slice()
            ],
            |row| row.get(0),
        )
        .expect("count rebuilt rollups");
    assert_eq!(rollup_count, 2);
    drop(connection);
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Converts a lifecycle status into the normalized SQLite label.
///
/// # Parameters
/// - `status`: Canonical plan-node status.
///
/// # Returns
/// The stable lowercase label written by the session repository.
fn status_tag(status: PlanNodeStatus) -> &'static str {
    match status {
        PlanNodeStatus::Planned => "planned",
        PlanNodeStatus::Ready => "ready",
        PlanNodeStatus::Running => "running",
        PlanNodeStatus::Completed => "completed",
        PlanNodeStatus::Failed => "failed",
        PlanNodeStatus::Abandoned => "abandoned",
    }
}

/// Creates a planned node with the requested structural parent.
///
/// # Parameters
/// - `id`: Stable node identity.
/// - `parent`: Branch or node parent.
///
/// # Returns
/// A minimal planned node for the index test.
fn node(id: EntityId, parent: PlanNodeParent) -> PlanNode {
    PlanNode {
        id,
        parent: Some(parent),
        title: "indexed node".to_owned(),
        purpose: "exercise normalized index".to_owned(),
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
