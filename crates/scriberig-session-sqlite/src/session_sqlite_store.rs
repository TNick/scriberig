//! Implements the provider-neutral session repository over SQLite.

use rusqlite::{TransactionBehavior, params};
use scriberig_core::EntityId;
use scriberig_session::{PlanStoreError, SelectionStoreError, Session, SessionStore};
use scriberig_store::{RootStore, SnapshotStore};

use super::{OptionalRow, SqliteSessionStore, payload};

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
        let mut rewrites = Vec::new();
        for (session_id, payload) in records {
            let mut decoded = payload::decode(&payload)?;
            self.hydrate_semantic_plans(&mut decoded.session)?;
            if decoded.requires_rewrite {
                rewrites.push((
                    session_id,
                    payload::encode(&decoded.session, &decoded.legacy_execution)?,
                ));
            }
            sessions.push(decoded.session);
        }
        if !rewrites.is_empty() {
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| error.to_string())?;
            for (session_id, sanitized) in rewrites {
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
        let mut decoded = payload::decode(&payload)?;
        self.hydrate_semantic_plans(&mut decoded.session)?;
        if !decoded.requires_rewrite {
            return Ok(Some(decoded.session));
        }
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let sanitized = payload::encode(&decoded.session, &decoded.legacy_execution)?;
        transaction
            .execute(
                "UPDATE session_records SET payload=?2 WHERE session_id=?1 AND payload=?3",
                params![session.as_bytes().as_slice(), sanitized, payload],
            )
            .map_err(|error| error.to_string())?;
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
        let prior_payload = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| "session repository lock poisoned".to_owned())?;
            connection
                .query_row(
                    "SELECT payload FROM session_records WHERE session_id=?1",
                    params![session.id.as_bytes().as_slice()],
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
        };
        let prior = prior_payload.as_deref().map(payload::decode).transpose()?;
        let legacy_execution = prior
            .as_ref()
            .map(|decoded| decoded.legacy_execution.clone())
            .unwrap_or_default();
        let mut projection = self.prepare_session_projection(session, prior)?;
        self.hydrate_semantic_plans(&mut projection)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let payload = payload::encode(&projection, &legacy_execution)?;
        transaction
            .execute(
                "INSERT INTO session_records(session_id, payload) VALUES (?1, ?2)
                 ON CONFLICT(session_id) DO UPDATE SET payload=excluded.payload",
                params![session.id.as_bytes().as_slice(), payload],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())
    }

    shared_session_plan_read_methods!();

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

    /// Lists sanitized unresolved legacy execution choices for operator review.
    ///
    /// - `session`: Stable identity of the session to inspect.
    ///
    /// Unresolved choices with their exact source paths.
    ///
    /// # Returns
    /// The unresolved choices or a backend diagnostic.
    ///
    /// # Errors
    /// Returns a diagnostic when the session payload cannot be read or decoded.
    fn legacy_execution_records(
        &self,
        session: EntityId,
    ) -> Result<Vec<scriberig_session::LegacyExecutionRecord>, String> {
        SqliteSessionStore::legacy_execution_records(self, session)
    }

    /// Atomically applies a validated session selection and clears its matching legacy record.
    ///
    /// - `session`: Complete session carrying the proposed execution selection.
    /// - `expected_revision`: Selection revision observed by the caller.
    /// - `legacy_path`: Exact source path of the legacy session selection.
    ///
    /// Unit after the mapped selection is persisted and its recovery record removed.
    ///
    /// # Returns
    /// Unit on success.
    ///
    /// # Errors
    /// Returns a revision conflict, missing legacy record, or persistence error.
    fn resolve_legacy_execution_selection(
        &self,
        session: &Session,
        expected_revision: u64,
        legacy_path: &str,
    ) -> Result<(), SelectionStoreError> {
        self.resolve_legacy_execution_selection_inner(session, expected_revision, legacy_path)
    }

    /// Removes an exact current plan recovery record after plan assignment persistence.
    ///
    /// - `session`: Stable session identity containing the record.
    /// - `legacy_path`: Exact path returned by recovery inspection.
    /// - `target`: Exact stable target observed before the plan mutation.
    ///
    /// Unit after the recovery record is removed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the record is absent or cannot be persisted.
    fn resolve_legacy_plan_assignment(
        &self,
        session: scriberig_core::EntityId,
        legacy_path: &str,
        target: &scriberig_session::LegacyExecutionTarget,
    ) -> Result<(), SelectionStoreError> {
        self.resolve_legacy_plan_assignment_inner(session, legacy_path, target)
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
