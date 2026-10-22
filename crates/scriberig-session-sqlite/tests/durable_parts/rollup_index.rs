//! Verifies rollup-index schema migration and snapshot revision binding.

use super::*;

/// Adds node-revision indexing when opening a database with the old cache schema.
#[test]
fn existing_rollup_index_schema_gains_node_revision_column() {
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
             );",
        )
        .expect("create old schema");
    drop(connection);

    let store = SqliteSessionStore::open(&path).expect("open upgraded repository");
    drop(store);
    let connection = rusqlite::Connection::open(&path).expect("reopen database");
    let has_node_revision = connection
        .query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM pragma_table_info('plan_node_rollup_index')
                 WHERE name='node_revision'
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .expect("inspect rollup schema");

    assert!(has_node_revision);
    let _ = std::fs::remove_file(path);
}
