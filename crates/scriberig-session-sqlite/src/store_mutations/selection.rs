//! Persists optimistic execution-selection changes.

use rusqlite::{TransactionBehavior, params};
use scriberig_session::{SelectionStoreError, Session, SessionExecutionSelection};

use crate::{OptionalRow, SqliteSessionStore, payload};

impl SqliteSessionStore {
    /// Atomically replaces only the execution selection at its observed revision.
    ///
    /// - `session`: Session snapshot with the proposed selection.
    /// - `expected_revision`: Revision observed before this mutation.
    ///
    /// # Returns
    /// Unit after the selection and preserved session graph are committed.
    ///
    /// # Errors
    /// Returns a revision conflict or bounded persistence failure.
    pub(super) fn compare_and_swap_execution_selection_inner(
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
        let decoded = payload
            .map(|value| payload::decode(&value).map_err(SelectionStoreError::Persistence))
            .transpose()?;
        let legacy_execution = decoded
            .as_ref()
            .map(|value| value.legacy_execution.clone())
            .unwrap_or_default();
        let stored = decoded.map(|value| value.session);
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
        let payload = payload::encode(&stored, &legacy_execution)
            .map_err(SelectionStoreError::Persistence)?;
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
}
