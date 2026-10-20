//! Exposes unresolved legacy selection data for deliberate operator recovery.

use rusqlite::{TransactionBehavior, params};
use scriberig_core::EntityId;

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
}
