//! Builds compact SQLite session read projections without plan history hydration.

use rusqlite::{OptionalExtension, TransactionBehavior, params};
use scriberig_core::EntityId;
use scriberig_session::Session;

use crate::{SqliteSessionStore, payload};

impl SqliteSessionStore {
    /// Loads one session and replaces aggregate plan values with compact semantic projections.
    ///
    /// Legacy payloads are sanitized and semantic plan roots are initialized as
    /// part of the same compatibility behavior as a complete session load.
    ///
    /// # Parameters
    /// - `self`: SQLite session repository to query.
    /// - `session`: Stable identity of the requested session.
    ///
    /// # Returns
    /// A session with current plan structure and summaries, but without
    /// unbounded history, gate, or evidence collections.
    ///
    /// # Errors
    /// Returns a diagnostic when the payload or semantic plan projection cannot be read.
    pub(super) fn load_session_read_projection(
        &self,
        session: EntityId,
    ) -> Result<Option<Session>, String> {
        let payload_bytes = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| "session repository lock poisoned".to_owned())?;
            connection
                .query_row(
                    "SELECT payload FROM session_records WHERE session_id=?1",
                    params![session.as_bytes().as_slice()],
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
        };
        let Some(payload_bytes) = payload_bytes else {
            return Ok(None);
        };
        let mut decoded = payload::decode(&payload_bytes)?;
        let rewrite_session = decoded.requires_rewrite.then(|| decoded.session.clone());
        self.hydrate_semantic_plan_projections(&mut decoded.session)?;
        if let Some(rewrite_session) = rewrite_session {
            let mut connection = self
                .connection
                .lock()
                .map_err(|_| "session repository lock poisoned".to_owned())?;
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| error.to_string())?;
            let sanitized = payload::encode(&rewrite_session, &decoded.legacy_execution)?;
            transaction
                .execute(
                    "UPDATE session_records SET payload=?2 WHERE session_id=?1 AND payload=?3",
                    params![session.as_bytes().as_slice(), sanitized, payload_bytes],
                )
                .map_err(|error| error.to_string())?;
            transaction.commit().map_err(|error| error.to_string())?;
        }
        Ok(Some(decoded.session))
    }

    /// Hydrates plan roots through the current compact semantic query.
    ///
    /// # Parameters
    /// - `self`: SQLite session repository to use.
    /// - `session`: Session whose plans are projected.
    ///
    /// # Returns
    /// Success after all compact plan projections are refreshed.
    ///
    /// # Errors
    /// Returns an error when migration or current plan projection fails.
    fn hydrate_semantic_plan_projections(&self, session: &mut Session) -> Result<(), String> {
        let mut plans = Vec::with_capacity(session.plans.len());
        for aggregate in &session.plans {
            self.migrate_plan_history(session.id, aggregate)?;
            plans.push(
                self.load_plan_projection(session.id, aggregate.id, None)?
                    .ok_or_else(|| format!("semantic plan {} disappeared", aggregate.id))?,
            );
        }
        session.plans = plans;
        Ok(())
    }
}
