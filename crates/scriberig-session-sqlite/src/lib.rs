//! Persists complete provider-neutral session graphs in SQLite.

mod payload;
#[macro_use]
#[path = "session_sqlite_plan_reads.rs"]
mod plan_reads;
#[path = "store_mutations/plan.rs"]
mod plan_mutation;
mod recovery;
mod rollup_index;
#[path = "store_mutations/selection.rs"]
mod selection_mutation;
mod semantic_plans;
#[path = "session_sqlite_projection.rs"]
mod session_projection;
mod session_sqlite_store;
pub use scriberig_session::LegacyExecutionRecord;

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::{Connection, OpenFlags};
use scriberig_store_sqlite::SqliteStore;

/// SQLite-backed repository for immutable session graph snapshots.
pub struct SqliteSessionStore {
    /// Connection guarded for serialized repository operations.
    pub(crate) connection: Mutex<Connection>,
    /// Universal immutable store used as canonical plan persistence.
    semantic_store: SqliteStore,
}

static NEXT_MEMORY_DATABASE: AtomicU64 = AtomicU64::new(1);

impl SqliteSessionStore {
    /// Writes a raw historical session payload for an integration fixture.
    ///
    /// This test-support-only method keeps direct SQL access inside the SQLite
    /// backend while allowing migration tests to install legacy wire shapes.
    ///
    /// # Parameters
    /// - `session`: Exact session identity used by the fixture row.
    /// - `payload`: Serialized legacy payload bytes to install.
    ///
    /// # Returns
    /// Success after the raw payload is stored.
    ///
    /// # Errors
    /// Returns a SQLite error when the fixture cannot be written.
    #[cfg(feature = "test-support")]
    pub fn write_legacy_payload_for_test(
        &self,
        session: scriberig_core::EntityId,
        payload: &[u8],
    ) -> rusqlite::Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.execute(
            "UPDATE session_records SET payload=?2 WHERE session_id=?1",
            rusqlite::params![session.as_bytes().as_slice(), payload],
        )?;
        Ok(())
    }

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
        Self::from_connections(Connection::open(path)?, SqliteStore::open(path)?)
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
        let id = NEXT_MEMORY_DATABASE.fetch_add(1, Ordering::Relaxed);
        let uri = format!("file:scriberig-session-{id}?mode=memory&cache=shared");
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_SHARED_CACHE;
        Self::from_connections(
            Connection::open_with_flags(&uri, flags)?,
            SqliteStore::open_with_flags(&uri, flags)?,
        )
    }

    /// Initializes the repository schema and universal semantic store.
    ///
    /// - `connection`: SQLite connection to initialize.
    /// - `semantic_store`: Universal semantic store used for exact plan records.
    ///
    /// An initialized session repository.
    ///
    /// Returns a SQLite schema error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn from_connections(
        connection: Connection,
        semantic_store: SqliteStore,
    ) -> rusqlite::Result<Self> {
        connection.execute_batch("PRAGMA busy_timeout = 5000;")?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS session_records (
                 session_id BLOB PRIMARY KEY,
                 payload BLOB NOT NULL
             );
             CREATE TABLE IF NOT EXISTS plan_node_rollup_index (
                 session_id TEXT NOT NULL,
                 plan_id TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 parent_node_id TEXT,
                 node_revision TEXT NOT NULL,
                 direct_status TEXT NOT NULL,
                 effective_status TEXT NOT NULL,
                 source_snapshot TEXT NOT NULL,
                 evaluator_version INTEGER NOT NULL,
                 payload BLOB NOT NULL,
                 PRIMARY KEY (session_id, plan_id, node_id)
             );",
        )?;
        let has_node_revision = {
            let mut statement = connection.prepare("PRAGMA table_info(plan_node_rollup_index)")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            columns
                .collect::<rusqlite::Result<Vec<_>>>()?
                .iter()
                .any(|column| column == "node_revision")
        };
        if !has_node_revision {
            connection.execute(
                "ALTER TABLE plan_node_rollup_index
                 ADD COLUMN node_revision TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
            semantic_store,
        })
    }
}

/// Imports the optional rusqlite row helper without exposing it publicly.
trait OptionalRow {
    /// Converts a missing row into `None`.
    ///
    /// The row payload when present.
    ///
    /// Returns the original SQLite query error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn optional(self) -> rusqlite::Result<Option<Vec<u8>>>;
}

impl OptionalRow for rusqlite::Result<Vec<u8>> {
    /// Converts query-row not-found into an optional payload.
    ///
    /// The payload when the query returned a row.
    ///
    /// Returns a non-not-found SQLite query error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn optional(self) -> rusqlite::Result<Option<Vec<u8>>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
