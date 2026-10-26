//! Verifies atomic mapping of one unresolved legacy session selection.

use rusqlite::{Connection, params};
use scriberig_core::EntityId;
use scriberig_session::{Branch, SelectionStoreError, Session, SessionStore};
use scriberig_session_sqlite::SqliteSessionStore;

/// Maps one preserved session choice and removes its recovery record atomically.
#[test]
fn legacy_session_mapping_validates_revision_and_clears_exact_record() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-session-legacy-map-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([89; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let session = Session {
        id: session_id,
        branches: vec![Branch::new(EntityId::from_bytes([90; 16]), "root").expect("root")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    let mut raw = serde_json::to_value(&session).expect("serialize session");
    raw["execution"] = serde_json::json!({
        "revision": 4,
        "model_profile": "retired-profile",
        "model": "retired-model",
        "thinking_level": "deep",
        "history": []
    });
    insert_raw_session(&path, session_id, &raw);
    let mut mapped = store
        .load(session_id)
        .expect("load imported session")
        .expect("session exists");
    assert_eq!(
        store
            .legacy_execution_records(session_id)
            .expect("list unresolved choices")[0]
            .path,
        "/execution"
    );
    mapped
        .execution
        .replace(
            4,
            Some("provider-current".to_owned()),
            Some("model-current".to_owned()),
            Some("medium".to_owned()),
        )
        .expect("make current selection");

    store
        .resolve_legacy_execution_selection(&mapped, 3, "/execution")
        .expect_err("stale selection revision is rejected");
    store
        .resolve_legacy_execution_selection(&mapped, 4, "/execution")
        .expect("commit exact mapping");

    let recovered = store
        .load(session_id)
        .expect("reload mapped session")
        .expect("session remains");
    assert_eq!(
        recovered.execution.provider_id.as_deref(),
        Some("provider-current")
    );
    assert_eq!(
        recovered.execution.model_id.as_deref(),
        Some("model-current")
    );
    assert_eq!(
        recovered.execution.thinking_level.as_deref(),
        Some("medium")
    );
    assert_eq!(
        store
            .legacy_execution_records(session_id)
            .expect("list remaining choices")
            .len(),
        0
    );
    assert_eq!(
        store.resolve_legacy_execution_selection(&mapped, 4, "/execution"),
        Err(SelectionStoreError::LegacyRecordNotFound)
    );
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Inserts one serialized legacy session row for recovery testing.
///
/// - `path`: SQLite database path initialized by the session store.
/// - `session`: Stable session identity for the row key.
/// - `payload`: Legacy session payload to store.
fn insert_raw_session(path: &std::path::Path, session: EntityId, payload: &serde_json::Value) {
    let connection = Connection::open(path).expect("open test connection");
    connection
        .execute(
            "INSERT INTO session_records(session_id, payload) VALUES (?1, ?2)",
            params![
                session.as_bytes().as_slice(),
                serde_json::to_vec(payload).expect("serialize legacy payload")
            ],
        )
        .expect("insert legacy record");
}
