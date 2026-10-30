//! Verifies durable branch results are compacted before session persistence.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_session::{
    Branch, BranchCandidateContext, BranchResult, BranchStatus, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;

/// Persists only bounded branch-result summaries and semantic handles.
#[test]
fn oversized_branch_results_are_compacted_before_durable_storage() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-result-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([41; 16]);
    let branch_id = EntityId::from_bytes([42; 16]);
    let mut branch = Branch::new(branch_id, "root").expect("root branch");
    branch.result = Some(BranchResult {
        id: EntityId::from_bytes([43; 16]),
        branch: branch_id,
        status: BranchStatus::Completed,
        summary: Some("summary".repeat(500)),
        result_kind: Some("agent".to_owned()),
        candidate_context: Some(BranchCandidateContext {
            project: ProjectId::from_bytes([44; 16]),
            base_snapshot: SnapshotId::new([45; 32]),
            candidate_snapshot: SnapshotId::new([46; 32]),
        }),
        references: Vec::new(),
        result_handles: (0..80).map(|index| format!("feature:{index}")).collect(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    let store = SqliteSessionStore::open(&path).expect("open session store");
    store
        .save(&Session {
            id: session_id,
            branches: vec![branch],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save bounded session");

    let loaded = store
        .load(session_id)
        .expect("load session")
        .expect("persisted session");
    let result = loaded.branches[0]
        .result
        .as_ref()
        .expect("persisted branch result");
    assert!(result.summary.as_ref().expect("summary").chars().count() <= 2_048);
    assert!(
        result
            .summary
            .as_ref()
            .expect("summary")
            .contains("branch result handles omitted")
    );
    assert_eq!(result.result_handles.len(), 32);
    assert!(result.result_handles.contains(&"feature:0".to_owned()));
    assert!(result.result_handles.contains(&"feature:31".to_owned()));
    assert_eq!(
        result.candidate_context,
        Some(BranchCandidateContext {
            project: ProjectId::from_bytes([44; 16]),
            base_snapshot: SnapshotId::new([45; 32]),
            candidate_snapshot: SnapshotId::new([46; 32]),
        })
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}
