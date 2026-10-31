//! Stores restart-safe markers for legacy plan migration recovery.

use rusqlite::params;
use scriberig_core::EntityId;

use crate::SqliteSessionStore;

impl SqliteSessionStore {
    /// Reads whether a legacy plan migration must finish claim recovery.
    ///
    /// # Parameters
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    ///
    /// Whether migration recovery remains pending.
    ///
    /// # Returns
    /// `true` when an interrupted import must resume recovery.
    ///
    /// # Errors
    /// Returns a bounded SQLite diagnostic when the marker cannot be read.
    pub(super) fn legacy_migration_pending(
        &self,
        session: EntityId,
        plan: EntityId,
    ) -> Result<bool, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM plan_legacy_migration_pending
                 WHERE session_id=?1 AND plan_id=?2)",
                params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())
    }

    /// Persists or clears the durable legacy migration recovery marker.
    ///
    /// # Parameters
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    /// - `pending`: Whether recovery must resume after process interruption.
    ///
    /// Unit after the migration marker is updated.
    ///
    /// # Returns
    /// Unit when the marker update succeeds.
    ///
    /// # Errors
    /// Returns a bounded SQLite diagnostic when the marker cannot be updated.
    pub(super) fn set_legacy_migration_pending(
        &self,
        session: EntityId,
        plan: EntityId,
        pending: bool,
    ) -> Result<(), String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        if pending {
            connection
                .execute(
                    "INSERT OR IGNORE INTO plan_legacy_migration_pending
                     (session_id, plan_id) VALUES (?1, ?2)",
                    params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
                )
                .map_err(|error| error.to_string())?;
        } else {
            connection
                .execute(
                    "DELETE FROM plan_legacy_migration_pending
                     WHERE session_id=?1 AND plan_id=?2",
                    params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
                )
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}
