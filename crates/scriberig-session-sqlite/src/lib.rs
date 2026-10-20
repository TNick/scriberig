//! Persists complete provider-neutral session graphs in SQLite.

mod payload;
#[path = "store_mutations/plan.rs"]
mod plan_mutation;
mod recovery;
#[path = "store_mutations/selection.rs"]
mod selection_mutation;

pub use payload::LegacyExecutionRecord;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, TransactionBehavior, params};
use scriberig_core::EntityId;
use scriberig_session::{PlanStoreError, SelectionStoreError, Session, SessionStore};

/// SQLite-backed repository for immutable session graph snapshots.
pub struct SqliteSessionStore {
    /// Connection guarded for serialized repository operations.
    connection: Mutex<Connection>,
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
        Self::from_connection(Connection::open(path)?)
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
        Self::from_connection(Connection::open_in_memory()?)
    }

    /// Initializes the repository schema on one connection.
    ///
    /// - `connection`: SQLite connection to initialize.
    ///
    /// An initialized session repository.
    ///
    /// Returns a SQLite schema error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// Loads all stored sessions in deterministic identity order.
    ///
    /// - `self`: SQLite session repository.
    ///
    /// Complete stored session graphs ordered by identity.
    ///
    /// Legacy selection rows are rewritten to their sanitized recovery envelope.
    ///
    /// Returns a bounded SQLite or serialization diagnostic.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when stored sessions cannot be read or decoded.
    fn list(&self) -> Result<Vec<Session>, String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let records = {
            let mut statement = connection
                .prepare("SELECT session_id, payload FROM session_records ORDER BY session_id")
                .map_err(|error| error.to_string())?;
            statement
                .query_map([], |row| {
                    Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?))
                })
                .map_err(|error| error.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| error.to_string())?
        };
        let mut sessions = Vec::with_capacity(records.len());
        let mut legacy_ids = Vec::new();
        for (session_id, payload) in records {
            let decoded = payload::decode(&payload)?;
            if decoded.requires_rewrite {
                legacy_ids.push(session_id);
            }
            sessions.push(decoded.session);
        }
        if !legacy_ids.is_empty() {
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| error.to_string())?;
            for session_id in legacy_ids {
                let payload = transaction
                    .query_row(
                        "SELECT payload FROM session_records WHERE session_id=?1",
                        params![session_id],
                        |row| row.get::<_, Vec<u8>>(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?;
                let Some(payload) = payload else {
                    continue;
                };
                let decoded = payload::decode(&payload)?;
                if !decoded.requires_rewrite {
                    continue;
                }
                let sanitized = payload::encode(&decoded.session, &decoded.legacy_execution)?;
                transaction
                    .execute(
                        "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                        params![session_id, sanitized],
                    )
                    .map_err(|error| error.to_string())?;
            }
            transaction.commit().map_err(|error| error.to_string())?;
        }
        Ok(sessions)
    }

    /// Loads and decodes one stored session graph.
    ///
    /// - `session`: Session identity to load.
    ///
    /// The decoded graph when present.
    ///
    /// Legacy selection rows are rewritten to their sanitized recovery envelope.
    ///
    /// Returns a bounded SQLite or serialization diagnostic.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn load(&self, session: EntityId) -> Result<Option<Session>, String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let payload = connection
            .query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some(payload) = payload else {
            return Ok(None);
        };
        let decoded = payload::decode(&payload)?;
        if !decoded.requires_rewrite {
            return Ok(Some(decoded.session));
        }
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let current_payload = transaction
            .query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some(current_payload) = current_payload else {
            transaction.commit().map_err(|error| error.to_string())?;
            return Ok(None);
        };
        let decoded = payload::decode(&current_payload)?;
        if decoded.requires_rewrite {
            let sanitized = payload::encode(&decoded.session, &decoded.legacy_execution)?;
            transaction
                .execute(
                    "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                    params![session.as_bytes().as_slice(), sanitized],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(Some(decoded.session))
    }

    /// Removes one exact session graph from the SQLite repository.
    ///
    /// - `session`: Stable session identity to remove.
    ///
    /// Unit after the record is removed; absent identities are already removed.
    ///
    /// Returns a bounded SQLite or repository-lock diagnostic.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when SQLite cannot delete the record.
    fn remove(&self, session: EntityId) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        connection
            .execute(
                "DELETE FROM session_records WHERE session_id=?1",
                params![session.as_bytes().as_slice()],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Encodes and atomically replaces one stored session graph.
    ///
    /// - `session`: Complete graph to persist.
    ///
    /// Returns a bounded serialization or SQLite diagnostic.
    ///
    /// Unit after the graph is durably replaced.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn save(&self, session: &Session) -> Result<(), String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let prior_payload = transaction
            .query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.id.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let legacy_execution = prior_payload
            .map(|payload| payload::decode(&payload).map(|decoded| decoded.legacy_execution))
            .transpose()?
            .unwrap_or_default();
        let payload = payload::encode(session, &legacy_execution)?;
        transaction
            .execute(
                "INSERT INTO session_records(session_id, payload) VALUES (?1, ?2)
                 ON CONFLICT(session_id) DO UPDATE SET payload=excluded.payload",
                params![session.id.as_bytes().as_slice(), payload],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    }

    /// Atomically changes only session execution selection at the observed revision.
    ///
    /// - `session`: Session snapshot with the proposed selection.
    /// - `expected_revision`: Revision observed before this mutation.
    ///
    /// Unit after the selection and preserved session graph are committed.
    ///
    /// Returns a revision conflict or bounded persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_swap_execution_selection(
        &self,
        session: &Session,
        expected_revision: u64,
    ) -> Result<(), SelectionStoreError> {
        self.compare_and_swap_execution_selection_inner(session, expected_revision)
    }

    /// Atomically replaces one plan without overwriting concurrent branch updates.
    /// New branch records in the proposal are merged in the same transaction.
    ///
    /// - `session`: Snapshot containing the proposed plan.
    /// - `plan_id`: Stable plan identity being replaced.
    /// - `expected_revision`: Plan revision observed by the caller.
    ///
    /// Returns a plan revision conflict or bounded persistence failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_swap_plan(
        &self,
        session: &Session,
        plan_id: EntityId,
        expected_revision: u64,
    ) -> Result<(), PlanStoreError> {
        self.compare_and_swap_plan_inner(session, plan_id, expected_revision)
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
