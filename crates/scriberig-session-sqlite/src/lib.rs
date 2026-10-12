//! Persists complete provider-neutral session graphs in SQLite.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, params};
use scriberig_core::EntityId;
use scriberig_session::{Session, SessionStore};

/// SQLite-backed repository for immutable session graph snapshots.
pub struct SqliteSessionStore {
    /// Connection guarded for serialized repository operations.
    connection: Mutex<Connection>,
}

impl SqliteSessionStore {
    /// Opens or creates a session repository at `path`.
    ///
    /// # Parameters
    /// - `path`: SQLite database path.
    ///
    /// # Returns
    /// An initialized session repository.
    ///
    /// # Errors
    /// Returns a SQLite initialization error.
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    /// Creates an isolated in-memory session repository.
    ///
    /// # Returns
    /// An initialized repository.
    ///
    /// # Errors
    /// Returns a SQLite initialization error.
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    /// Initializes the repository schema on one connection.
    ///
    /// # Parameters
    /// - `connection`: SQLite connection to initialize.
    ///
    /// # Returns
    /// An initialized session repository.
    ///
    /// # Errors
    /// Returns a SQLite schema error.
    fn from_connection(connection: Connection) -> rusqlite::Result<Self> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS session_records (
                 session_id BLOB PRIMARY KEY,
                 payload BLOB NOT NULL
             );",
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
}

impl SessionStore for SqliteSessionStore {
    /// Loads and decodes one stored session graph.
    ///
    /// # Parameters
    /// - `session`: Session identity to load.
    ///
    /// # Returns
    /// The decoded graph when present.
    ///
    /// # Errors
    /// Returns a bounded SQLite or serialization diagnostic.
    fn load(&self, session: EntityId) -> Result<Option<Session>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let mut statement = connection
            .prepare("SELECT payload FROM session_records WHERE session_id=?1")
            .map_err(|error| error.to_string())?;
        let payload = statement
            .query_row(params![session.as_bytes().as_slice()], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .optional()
            .map_err(|error| error.to_string())?;
        payload
            .map(|payload| serde_json::from_slice(&payload).map_err(|error| error.to_string()))
            .transpose()
    }

    /// Encodes and atomically replaces one stored session graph.
    ///
    /// # Parameters
    /// - `session`: Complete graph to persist.
    ///
    /// # Errors
    /// Returns a bounded serialization or SQLite diagnostic.
    ///
    /// # Returns
    /// Unit after the graph is durably replaced.
    fn save(&self, session: &Session) -> Result<(), String> {
        let payload = serde_json::to_vec(session).map_err(|error| error.to_string())?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        connection
            .execute(
                "INSERT INTO session_records(session_id, payload) VALUES (?1, ?2)
                 ON CONFLICT(session_id) DO UPDATE SET payload=excluded.payload",
                params![session.id.as_bytes().as_slice(), payload],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

/// Imports the optional rusqlite row helper without exposing it publicly.
trait OptionalRow {
    /// Converts a missing row into `None`.
    ///
    /// # Returns
    /// The row payload when present.
    ///
    /// # Errors
    /// Returns the original SQLite query error.
    fn optional(self) -> rusqlite::Result<Option<Vec<u8>>>;
}

impl OptionalRow for rusqlite::Result<Vec<u8>> {
    /// Converts query-row not-found into an optional payload.
    ///
    /// # Returns
    /// The payload when the query returned a row.
    ///
    /// # Errors
    /// Returns a non-not-found SQLite query error.
    fn optional(self) -> rusqlite::Result<Option<Vec<u8>>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
