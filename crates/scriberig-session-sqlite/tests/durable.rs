//! Verifies session graphs survive repository reopen and preserve child state.

use scriberig_core::EntityId;
use scriberig_session::{
    Branch, CacheDisposition, ModelInvocation, SelectionStoreError, Session, SessionStore,
};
use scriberig_session_sqlite::SqliteSessionStore;
use std::sync::Arc;

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
            execution: scriberig_session::SessionExecutionSelection::default(),
        };
        session
            .add_child_branch(child_id, root_id, "child", "inspect")
            .expect("child");
        session
            .execution
            .replace(
                0,
                Some("profile-a".to_owned()),
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
    assert_eq!(loaded.execution.model.as_deref(), Some("model-a"));
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

/// Verifies selection compare-and-swap preserves concurrent branch updates.
#[test]
fn execution_selection_compare_and_swap_is_atomic() {
    let session_id = EntityId::from_bytes([11; 16]);
    let root_id = EntityId::from_bytes([12; 16]);
    let child_id = EntityId::from_bytes([13; 16]);
    let store = Arc::new(SqliteSessionStore::open_in_memory().expect("open store"));
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial session");
    let mut proposed = store
        .load(session_id)
        .expect("load selection snapshot")
        .expect("session exists");
    proposed
        .execution
        .replace(
            0,
            Some("profile-a".to_owned()),
            Some("model-a".to_owned()),
            Some("careful".to_owned()),
        )
        .expect("propose selection");
    proposed
        .execution
        .history
        .push(scriberig_session::SessionExecutionChange {
            revision: proposed.execution.revision,
            kind: scriberig_session::AgentActivityKind::ModelSelectionChanged,
            summary: "Model selected: model-a".to_owned(),
            model: proposed.execution.model.clone(),
            thinking_level: proposed.execution.thinking_level.clone(),
        });
    let mut branch_update = store
        .load(session_id)
        .expect("load branch snapshot")
        .expect("session exists");
    branch_update
        .branches
        .push(Branch::new(child_id, "new branch").expect("child"));
    store.save(&branch_update).expect("save concurrent branch");
    branch_update
        .execution
        .replace(
            0,
            Some("profile-b".to_owned()),
            Some("model-b".to_owned()),
            Some("steady".to_owned()),
        )
        .expect("propose competing selection");

    let first_store = Arc::clone(&store);
    let first =
        std::thread::spawn(move || first_store.compare_and_swap_execution_selection(&proposed, 0));
    let second_store = Arc::clone(&store);
    let second = std::thread::spawn(move || {
        second_store.compare_and_swap_execution_selection(&branch_update, 0)
    });
    let outcomes = [
        first.join().expect("first mutation"),
        second.join().expect("second mutation"),
    ];
    assert_eq!(outcomes.iter().filter(|value| value.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| matches!(value, Err(SelectionStoreError::RevisionConflict { .. })))
            .count(),
        1
    );
    let stored = store
        .load(session_id)
        .expect("reload session")
        .expect("session exists");
    assert_eq!(stored.branches.len(), 2);
    assert!(matches!(
        stored.execution.model.as_deref(),
        Some("model-a" | "model-b")
    ));
    assert_eq!(stored.execution.history.len(), 1);
    assert_eq!(
        stored.execution.history[0].model.as_deref(),
        stored.execution.model.as_deref()
    );
}
