//! Exposes unresolved legacy selection data for deliberate operator recovery.

use rusqlite::{TransactionBehavior, params};
use scriberig_core::EntityId;
use scriberig_session::{SelectionStoreError, Session};

use crate::{LegacyExecutionRecord, OptionalRow, SqliteSessionStore, payload};

impl SqliteSessionStore {
    /// Lists unresolved legacy model choices for explicit operator recovery.
    ///
    /// - `session`: Stable identity of the session record to inspect.
    ///
    /// Allowlisted legacy selection fields with their original JSON paths.
    ///
    /// Returns an empty list when the session has no unresolved legacy choice.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the record cannot be read or decoded.
    pub fn legacy_execution_records(
        &self,
        session: EntityId,
    ) -> Result<Vec<LegacyExecutionRecord>, String> {
        let connection = self
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
        payload
            .map(|payload| payload::decode(&payload).map(|decoded| decoded.legacy_execution))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    /// Discards preserved legacy choices after an explicit operator decision.
    ///
    /// - `session`: Stable identity of the session record to update.
    ///
    /// Unit after the recovery metadata is removed; absent sessions are unchanged.
    ///
    /// Returns an error when the record cannot be read, decoded, or written.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn discard_legacy_execution_records(&self, session: EntityId) -> Result<(), String> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let payload = transaction
            .query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some(payload) = payload else {
            return transaction.commit().map_err(|error| error.to_string());
        };
        let decoded = payload::decode(&payload)?;
        if decoded.legacy_execution.is_empty() {
            return transaction.commit().map_err(|error| error.to_string());
        }
        let payload = payload::encode(&decoded.session, &[])?;
        transaction
            .execute(
                "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                params![session.as_bytes().as_slice(), payload],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    }

    /// Resolves one legacy session selection with an already validated current choice.
    ///
    /// - `session`: Session containing the proposed next execution selection.
    /// - `expected_revision`: Selection revision observed by the caller.
    /// - `legacy_path`: Exact legacy source path selected for recovery.
    ///
    /// Unit after the new selection and recovery-record removal commit together.
    ///
    /// # Returns
    /// Unit on success.
    ///
    /// # Errors
    /// Returns a revision conflict, missing record, or persistence error.
    pub(crate) fn resolve_legacy_execution_selection_inner(
        &self,
        session: &Session,
        expected_revision: u64,
        legacy_path: &str,
    ) -> Result<(), SelectionStoreError> {
        if legacy_path != "/execution" {
            return Err(SelectionStoreError::Persistence(
                "only a session-level legacy selection can be mapped through this operation"
                    .to_owned(),
            ));
        }
        let mut connection = self.connection.lock().map_err(|_| {
            SelectionStoreError::Persistence("session repository lock poisoned".to_owned())
        })?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?;
        let raw = transaction
            .query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.id.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?
            .ok_or_else(|| {
                SelectionStoreError::Persistence("session record was not found".to_owned())
            })?;
        let decoded = payload::decode(&raw).map_err(SelectionStoreError::Persistence)?;
        let Some(record_index) = decoded
            .legacy_execution
            .iter()
            .position(|record| record.path == legacy_path)
        else {
            return Err(SelectionStoreError::LegacyRecordNotFound);
        };
        let actual_revision = decoded.session.execution.revision;
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
        if session.id != decoded.session.id {
            return Err(SelectionStoreError::Persistence(
                "session identity changed during legacy recovery".to_owned(),
            ));
        }
        let mut stored = decoded.session;
        stored.execution = session.execution.clone();
        let mut legacy = decoded.legacy_execution;
        legacy.remove(record_index);
        let encoded =
            payload::encode(&stored, &legacy).map_err(SelectionStoreError::Persistence)?;
        transaction
            .execute(
                "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                params![session.id.as_bytes().as_slice(), encoded],
            )
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))
    }

    /// Removes one exact non-claim plan-assignment record after its semantic write succeeds.
    ///
    /// - `session`: Stable owner of the SQLite recovery envelope.
    /// - `legacy_path`: Exact source path mapped by the operator.
    /// - `expected_target`: Exact stable plan/node target observed before mapping.
    ///
    /// Unit after the recovery record is removed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the record is absent, claim-owned, or unreadable.
    pub(crate) fn resolve_legacy_plan_assignment_inner(
        &self,
        session: EntityId,
        legacy_path: &str,
        expected_target: &scriberig_session::LegacyExecutionTarget,
    ) -> Result<(), SelectionStoreError> {
        let mut connection = self.connection.lock().map_err(|_| {
            SelectionStoreError::Persistence("session repository lock poisoned".to_owned())
        })?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?;
        let raw = transaction
            .query_row(
                "SELECT payload FROM session_records WHERE session_id=?1",
                params![session.as_bytes().as_slice()],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?
            .ok_or_else(|| {
                SelectionStoreError::Persistence("session record was not found".to_owned())
            })?;
        let decoded = payload::decode(&raw).map_err(SelectionStoreError::Persistence)?;
        let Some(record_index) = decoded.legacy_execution.iter().position(|record| {
            record.path == legacy_path
                && record.target.as_ref() == Some(expected_target)
                && !expected_target.claim
        }) else {
            return Err(SelectionStoreError::LegacyRecordNotFound);
        };
        let mut legacy = decoded.legacy_execution;
        legacy.remove(record_index);
        let encoded =
            payload::encode(&decoded.session, &legacy).map_err(SelectionStoreError::Persistence)?;
        transaction
            .execute(
                "UPDATE session_records SET payload=?2 WHERE session_id=?1",
                params![session.as_bytes().as_slice(), encoded],
            )
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| SelectionStoreError::Persistence(error.to_string()))
    }
}
