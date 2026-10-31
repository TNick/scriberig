//! Verifies session selection compare-and-swap persistence.

use super::*;

/// Verifies selection CAS preserves concurrent branch updates.
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
            plans: Vec::new(),
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
            Some("provider-a".to_owned()),
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
            provider_id: proposed.execution.provider_id.clone(),
            model_id: proposed.execution.model_id.clone(),
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
            Some("provider-b".to_owned()),
            Some("model-b".to_owned()),
            Some("steady".to_owned()),
        )
        .expect("propose competing selection");
    branch_update
        .execution
        .history
        .push(scriberig_session::SessionExecutionChange {
            revision: branch_update.execution.revision,
            kind: scriberig_session::AgentActivityKind::ModelSelectionChanged,
            summary: "Model selected: model-b".to_owned(),
            provider_id: branch_update.execution.provider_id.clone(),
            model_id: branch_update.execution.model_id.clone(),
            thinking_level: branch_update.execution.thinking_level.clone(),
        });

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
        1,
        "unexpected selection CAS outcomes: {outcomes:?}"
    );
    let stored = store
        .load(session_id)
        .expect("reload session")
        .expect("session exists");
    assert_eq!(stored.branches.len(), 2);
    assert!(matches!(
        stored.execution.model_id.as_deref(),
        Some("model-a" | "model-b")
    ));
    assert_eq!(stored.execution.history.len(), 1);
    assert_eq!(
        stored.execution.history[0].provider_id.as_deref(),
        stored.execution.provider_id.as_deref()
    );
}
