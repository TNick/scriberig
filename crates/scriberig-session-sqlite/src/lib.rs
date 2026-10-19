//! Persists complete provider-neutral session graphs in SQLite.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, TransactionBehavior, params};
use scriberig_core::EntityId;
use scriberig_session::{
    PlanStoreError, SelectionStoreError, Session, SessionExecutionSelection, SessionStore,
};

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
    /// Returns a bounded SQLite or serialization diagnostic.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when stored sessions cannot be read or decoded.
    fn list(&self) -> Result<Vec<Session>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let mut statement = connection
            .prepare("SELECT payload FROM session_records ORDER BY session_id")
            .map_err(|error| error.to_string())?;
        let payloads = statement
            .query_map([], |row| row.get::<_, Vec<u8>>(0))
            .map_err(|error| error.to_string())?;
        payloads
            .map(|payload| {
                let payload = payload.map_err(|error| error.to_string())?;
                serde_json::from_slice(&payload).map_err(|error| error.to_string())
            })
            .collect()
    }

    /// Loads and decodes one stored session graph.
    ///
    /// - `session`: Session identity to load.
    ///
    /// The decoded graph when present.
    ///
    /// Returns a bounded SQLite or serialization diagnostic.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SelectionStoreError::Persistence("session lock poisoned".to_owned()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?;
        let payload = transaction
            .query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.id.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?;
        let stored = payload
            .map(|value| {
                serde_json::from_slice::<Session>(&value)
                    .map_err(|error| SelectionStoreError::Persistence(error.to_string()))
            })
            .transpose()?;
        let actual_revision = stored.as_ref().map_or(0, |value| value.execution.revision);
        if actual_revision != expected_revision {
            return Err(SelectionStoreError::RevisionConflict {
                expected: expected_revision,
                actual: actual_revision,
            });
        }
        let next_revision = expected_revision.checked_add(1).ok_or_else(|| {
            SelectionStoreError::Persistence("selection revision exhausted".to_owned())
        })?;
        if session.execution.revision != next_revision {
            return Err(SelectionStoreError::Persistence(
                "proposed selection has an unexpected revision".to_owned(),
            ));
        }
        let mut stored = stored.unwrap_or_else(|| Session {
            id: session.id,
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: SessionExecutionSelection::default(),
        });
        stored.execution = session.execution.clone();
        let payload = serde_json::to_vec(&stored)
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?;
        transaction
            .execute(
                "INSERT INTO session_records(session_id, payload) VALUES (?1, ?2)
                 ON CONFLICT(session_id) DO UPDATE SET payload=excluded.payload",
                params![session.id.as_bytes().as_slice(), payload],
            )
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))
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
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| PlanStoreError::Persistence("session lock poisoned".to_owned()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))?;
        let payload = transaction
            .query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.id.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))?
            .ok_or_else(|| PlanStoreError::Persistence("session is not stored".to_owned()))?;
        let mut stored = serde_json::from_slice::<Session>(&payload)
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))?;
        let stored_plan = stored
            .plans
            .iter_mut()
            .find(|plan| plan.id == plan_id)
            .ok_or_else(|| PlanStoreError::Persistence("plan is not stored".to_owned()))?;
        if stored_plan.revision != expected_revision {
            return Err(PlanStoreError::RevisionConflict {
                expected: expected_revision,
                actual: stored_plan.revision,
            });
        }
        let proposed = session
            .plans
            .iter()
            .find(|plan| plan.id == plan_id)
            .ok_or_else(|| PlanStoreError::Persistence("proposed plan is absent".to_owned()))?;
        if proposed.revision != expected_revision.saturating_add(1) {
            return Err(PlanStoreError::Persistence(
                "proposed plan has an unexpected revision".to_owned(),
            ));
        }
        *stored_plan = proposed.clone();
        for branch in &session.branches {
            if stored.branches.iter().any(|known| known.id == branch.id) {
                continue;
            }
            stored.branches.push(branch.clone());
            if let Some(parent_id) = branch.parent
                && let Some(parent) = stored
                    .branches
                    .iter_mut()
                    .find(|known| known.id == parent_id)
                && !parent.children.contains(&branch.id)
            {
                parent.children.push(branch.id);
            }
        }
        let payload = serde_json::to_vec(&stored)
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))?;
        transaction
            .execute(
                "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                params![session.id.as_bytes().as_slice(), payload],
            )
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| PlanStoreError::Persistence(error.to_string()))
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
