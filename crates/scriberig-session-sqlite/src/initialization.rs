//! Initializes the SQLite-backed session repository.

use std::path::Path;

use scriberig_store_sqlite::SqliteStore;

use crate::SqliteSessionStore;

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
        connection_guard.execute_batch(
            "CREATE TABLE IF NOT EXISTS session_records (
                 session_id BLOB PRIMARY KEY,
                 payload BLOB NOT NULL
             );",
        )?;
        drop(connection_guard);
        Ok(Self {
            connection,
            semantic_store,
        })
    }
}
