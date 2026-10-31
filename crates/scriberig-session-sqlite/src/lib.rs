//! Persists complete provider-neutral session graphs in SQLite.

mod branch_projection;
mod initialization;
mod payload;
#[path = "store_mutations/plan_indexes.rs"]
mod plan_indexes;
#[path = "store_mutations/plan.rs"]
mod plan_mutation;
mod plan_projection;
mod recovery;
mod repository;
#[path = "store_mutations/selection.rs"]
mod selection_mutation;
mod semantic_plans;
mod session_sqlite_projection;

pub use scriberig_session::LegacyExecutionRecord;

use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use scriberig_store_sqlite::SqliteStore;

/// SQLite-backed repository for immutable session graph snapshots.
pub struct SqliteSessionStore {
    /// Connection guarded for serialized repository operations.
    connection: Arc<Mutex<Connection>>,
    /// Universal immutable store used as canonical plan persistence.
    semantic_store: SqliteStore,
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

impl SqliteSessionStore {
    /// Writes a raw historical payload for a migration integration fixture.
    ///
    /// This feature-gated helper keeps direct SQL access within the durable
    /// storage crate.
    ///
    /// # Parameters
    /// - `self`: SQLite repository whose session table receives the payload.
    /// - `session`: Exact session identity for the fixture row.
    /// - `payload`: Serialized historical payload bytes.
    ///
    /// # Returns
    /// Success after the row payload is replaced.
    ///
    /// # Errors
    /// Returns a SQLite error when the fixture row cannot be updated.
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
}
