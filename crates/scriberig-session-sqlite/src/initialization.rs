//! Initializes the SQLite-backed session repository.

use std::path::Path;

use scriberig_store_sqlite::SqliteStore;

use crate::SqliteSessionStore;

/// Removes derived rollup tables whose schema predates the normalized cache.
///
/// Cached rollups can be reconstructed from canonical semantic records, so an
/// incompatible cache is discarded while session and semantic data stay intact.
///
/// # Parameters
/// - `connection`: Open SQLite connection to inspect and migrate.
///
/// # Returns
/// Unit when the cache schema is absent, compatible, or safely rebuilt.
///
/// # Errors
/// Returns an error when SQLite cannot inspect or update the cache schema.
fn migrate_rollup_cache_schema(connection: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(plan_node_rollup_index)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<std::collections::BTreeSet<_>>>()?;
    drop(statement);
    if columns.is_empty() {
        return Ok(());
    }
    let required = [
        "session_id",
        "plan_id",
        "node_id",
        "effective_status",
        "contains_ignored",
        "contains_abandoned",
        "child_planned",
        "child_ready",
        "child_running",
        "child_completed",
        "child_failed",
        "child_abandoned",
        "unsatisfied_completion_gates",
        "evaluator_version",
        "source_snapshot",
    ];
    if required.iter().any(|column| !columns.contains(*column)) {
        connection.execute_batch(
            "DROP TABLE plan_node_rollup_index;
             DROP TABLE IF EXISTS plan_node_rollup_meta;",
        )?;
        return Ok(());
    }
    for (column, definition) in [
        ("entry_pending", "INTEGER NOT NULL DEFAULT 0"),
        ("entry_satisfied", "INTEGER NOT NULL DEFAULT 0"),
        ("entry_failed", "INTEGER NOT NULL DEFAULT 0"),
        ("entry_waived", "INTEGER NOT NULL DEFAULT 0"),
        ("completion_pending", "INTEGER NOT NULL DEFAULT 0"),
        ("completion_satisfied", "INTEGER NOT NULL DEFAULT 0"),
        ("completion_failed", "INTEGER NOT NULL DEFAULT 0"),
        ("completion_waived", "INTEGER NOT NULL DEFAULT 0"),
        ("evidence_count", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        if !columns.contains(column) {
            connection.execute_batch(&format!(
                "ALTER TABLE plan_node_rollup_index ADD COLUMN {column} {definition}"
            ))?;
        }
    }
    Ok(())
}

impl SqliteSessionStore {
    /// Opens or creates a session repository at `path`.
    ///
    /// - `path`: SQLite database path.
    ///
    /// An initialized session repository.
    ///
    /// Returns a SQLite initialization error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let path = path.as_ref();
        let semantic_store = SqliteStore::open(path)?;
        Self::from_semantic_store(semantic_store)
    }

    /// Creates an isolated in-memory session repository.
    ///
    /// An initialized repository.
    ///
    /// Returns a SQLite initialization error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let semantic_store = SqliteStore::open_in_memory()?;
        Self::from_semantic_store(semantic_store)
    }

    /// Initializes the repository schema and universal semantic store.
    ///
    /// - `semantic_store`: Universal store for immutable semantic records.
    ///
    /// An initialized session repository.
    ///
    /// Returns a SQLite schema error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn from_semantic_store(semantic_store: SqliteStore) -> rusqlite::Result<Self> {
        let connection = semantic_store.shared_connection();
        let connection_guard = connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        migrate_rollup_cache_schema(&connection_guard)?;
        connection_guard.execute_batch(
            "CREATE TABLE IF NOT EXISTS session_records (
                 session_id BLOB PRIMARY KEY,
                 payload BLOB NOT NULL
             );
             CREATE TABLE IF NOT EXISTS session_branch_records (
                 session_id BLOB NOT NULL,
                 branch_id BLOB NOT NULL,
                 ordinal INTEGER NOT NULL,
                 payload BLOB NOT NULL,
                 PRIMARY KEY (session_id, branch_id)
             );
             CREATE TABLE IF NOT EXISTS session_branch_meta (
                 session_id BLOB PRIMARY KEY
             );
             CREATE TABLE IF NOT EXISTS session_plan_records (
                 session_id BLOB NOT NULL,
                 plan_id BLOB NOT NULL,
                 ordinal INTEGER NOT NULL,
                 PRIMARY KEY (session_id, plan_id)
             );
             CREATE INDEX IF NOT EXISTS session_plan_records_order
                 ON session_plan_records(session_id, ordinal, plan_id);
             CREATE TABLE IF NOT EXISTS plan_node_index (
                 session_id BLOB NOT NULL,
                 plan_id BLOB NOT NULL,
                 node_id BLOB NOT NULL,
                 parent_node_id BLOB,
                 parent_branch_id BLOB,
                 current_revision_id BLOB NOT NULL,
                 direct_status TEXT NOT NULL,
                 source_snapshot BLOB NOT NULL,
                 PRIMARY KEY (session_id, plan_id, node_id)
             );
             CREATE INDEX IF NOT EXISTS plan_node_index_parent
                 ON plan_node_index (session_id, plan_id, parent_node_id, node_id);
             CREATE TABLE IF NOT EXISTS plan_node_index_meta (
                 session_id BLOB NOT NULL,
                 plan_id BLOB NOT NULL,
                 source_snapshot BLOB NOT NULL,
                 node_count INTEGER NOT NULL,
                 PRIMARY KEY (session_id, plan_id)
             );
             CREATE TABLE IF NOT EXISTS plan_node_rollup_index (
                 session_id BLOB NOT NULL,
                 plan_id BLOB NOT NULL,
                 node_id BLOB NOT NULL,
                 effective_status TEXT NOT NULL,
                 contains_ignored INTEGER NOT NULL,
                 contains_abandoned INTEGER NOT NULL,
                 child_planned INTEGER NOT NULL,
                 child_ready INTEGER NOT NULL,
                 child_running INTEGER NOT NULL,
                 child_completed INTEGER NOT NULL,
                 child_failed INTEGER NOT NULL,
                 child_abandoned INTEGER NOT NULL,
                 unsatisfied_completion_gates INTEGER NOT NULL,
                 entry_pending INTEGER NOT NULL DEFAULT 0,
                 entry_satisfied INTEGER NOT NULL DEFAULT 0,
                 entry_failed INTEGER NOT NULL DEFAULT 0,
                 entry_waived INTEGER NOT NULL DEFAULT 0,
                 completion_pending INTEGER NOT NULL DEFAULT 0,
                 completion_satisfied INTEGER NOT NULL DEFAULT 0,
                 completion_failed INTEGER NOT NULL DEFAULT 0,
                 completion_waived INTEGER NOT NULL DEFAULT 0,
                 evidence_count INTEGER NOT NULL DEFAULT 0,
                 evaluator_version INTEGER NOT NULL,
                 source_snapshot BLOB NOT NULL,
                 PRIMARY KEY (session_id, plan_id, node_id)
             );
             CREATE TABLE IF NOT EXISTS plan_node_rollup_meta (
                 session_id BLOB NOT NULL,
                 plan_id BLOB NOT NULL,
                 source_snapshot BLOB NOT NULL,
                 evaluator_version INTEGER NOT NULL,
                 node_count INTEGER NOT NULL,
                 PRIMARY KEY (session_id, plan_id)
             );
             CREATE TABLE IF NOT EXISTS plan_legacy_migration_pending (
                 session_id BLOB NOT NULL,
                 plan_id BLOB NOT NULL,
                 PRIMARY KEY (session_id, plan_id)
             );",
        )?;
        drop(connection_guard);
        Ok(Self {
            connection,
            semantic_store,
        })
    }
}
