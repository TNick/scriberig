//! Verifies existing oversized branch results are compacted when read.

use super::payload;
use crate::SqliteSessionStore;
use rusqlite::params;
use scriberig_core::EntityId;
use scriberig_session::{
    Branch, BranchResult, BranchStatus, Session, SessionExecutionSelection, SessionStore,
};

/// Builds an oversized result payload for an existing durable session row.
///
/// # Parameters
/// - `session_id`: Stable identity for the containing session.
/// - `branch_id`: Stable identity for the branch holding the result.
///
/// # Returns
/// A session containing an intentionally oversized branch result.
fn legacy_session(session_id: EntityId, branch_id: EntityId) -> Session {
    let mut branch = Branch::new(branch_id, "root").expect("root branch");
    branch.result = Some(BranchResult {
        id: EntityId::new(),
        branch: branch_id,
        status: BranchStatus::Completed,
        summary: Some("legacy summary ".repeat(300)),
        result_kind: Some("agent".to_owned()),
        candidate_context: None,
        references: Vec::new(),
        result_handles: (0..80).map(|index| format!("feature:{index}")).collect(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    Session {
        id: session_id,
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: SessionExecutionSelection::default(),
    }
}

/// Rewrites old oversized results through both exact and list reads.
#[test]
fn legacy_oversized_results_are_compacted_during_load_and_list() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-legacy-result-{}.sqlite",
        EntityId::new()
    ));
    let store = SqliteSessionStore::open(&path).expect("open session store");
    let ids = [
        (
            EntityId::from_bytes([51; 16]),
            EntityId::from_bytes([52; 16]),
        ),
        (
            EntityId::from_bytes([53; 16]),
            EntityId::from_bytes([54; 16]),
        ),
    ];
    for (session_id, branch_id) in ids {
        let seed = Session {
            id: session_id,
            branches: vec![Branch::new(branch_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: SessionExecutionSelection::default(),
        };
        store.save(&seed).expect("create session row");
        let bytes = payload::encode(&legacy_session(session_id, branch_id), &[])
            .expect("encode legacy oversized result");
        rusqlite::Connection::open(&path)
            .expect("open raw session database")
            .execute(
                "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                params![session_id.as_bytes().as_slice(), bytes],
            )
            .expect("install legacy payload");
    }

    let loaded = store
        .load(ids[0].0)
        .expect("load and compact legacy result")
        .expect("session exists");
    assert_compact(&loaded);
    let listed = store.list().expect("list and compact legacy result");
    assert_eq!(listed.len(), ids.len());
    for session in &listed {
        assert_compact(session);
    }
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Asserts a loaded graph contains only bounded branch-result data.
///
/// # Parameters
/// - `session`: Loaded session graph to inspect.
fn assert_compact(session: &Session) {
    let result = session.branches[0]
        .result
        .as_ref()
        .expect("branch result remains inspectable");
    assert!(result.summary.as_ref().expect("summary").chars().count() <= 2_048);
    assert!(
        result
            .summary
            .as_ref()
            .expect("summary")
            .contains("branch result handles omitted")
    );
    assert_eq!(result.result_handles.len(), 32);
}
