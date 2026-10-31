//! Verifies rollup-index schema migration and snapshot revision binding.

use super::*;

/// Rebuilds an incompatible cache schema while retaining canonical session rows.
#[test]
fn existing_rollup_index_schema_is_replaced_without_losing_session_data() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-rollup-schema-{}.sqlite",
        EntityId::new()
    ));
    let connection = rusqlite::Connection::open(&path).expect("create prior database");
    connection
        .execute_batch(
            "CREATE TABLE session_records (
                 session_id BLOB PRIMARY KEY,
                 payload BLOB NOT NULL
             );
             CREATE TABLE plan_node_rollup_index (
                 session_id TEXT NOT NULL,
                 plan_id TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 parent_node_id TEXT,
                 direct_status TEXT NOT NULL,
                 effective_status TEXT NOT NULL,
                 source_snapshot TEXT NOT NULL,
                 evaluator_version INTEGER NOT NULL,
                 payload BLOB NOT NULL,
                 PRIMARY KEY (session_id, plan_id, node_id)
             );
             INSERT INTO session_records(session_id, payload) VALUES (x'01', x'02');",
        )
        .expect("create old schema");
    drop(connection);

    let store = SqliteSessionStore::open(&path).expect("open upgraded repository");
    drop(store);
    let connection = rusqlite::Connection::open(&path).expect("reopen database");
    let has_entry_gate_count = connection
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM pragma_table_info('plan_node_rollup_index')
                 WHERE name='entry_pending'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .expect("inspect rollup schema");
    let old_payload_column = connection
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM pragma_table_info('plan_node_rollup_index')
                 WHERE name='payload'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .expect("inspect old cache column");
    let session_row_survived = connection
        .query_row("SELECT COUNT(*) FROM session_records", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("inspect canonical session data");

    assert!(has_entry_gate_count);
    assert!(!old_payload_column);
    assert_eq!(session_row_survived, 1);
    let _ = std::fs::remove_file(path);
}
