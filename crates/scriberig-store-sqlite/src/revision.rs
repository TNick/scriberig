//! Durable revision persistence for the SQLite backend.

use crate::SqliteStore;
use rusqlite::{OptionalExtension, params};
use scriberig_core::{EncodingVersion, ObjectType, RevisionId};
use scriberig_store::{Revision, RevisionStore, StoreError};

/// Converts a SQLite error to a revision backend error.
///
/// # Parameters
/// - `error`: SQLite error.
///
/// # Returns
/// A typed revision backend error.
fn sql_store(error: rusqlite::Error) -> StoreError {
    StoreError::Backend(error.to_string())
}

impl RevisionStore for SqliteStore {
    /// # Parameters
    /// - `revision`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Inserts an immutable revision idempotently.
    fn put(&self, revision: Revision) -> Result<RevisionId, StoreError> {
        let id = revision.id();
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT OR IGNORE INTO revisions(id,version,object_type,payload) VALUES (?,?,?,?)",
                params![
                    id.as_bytes().as_slice(),
                    revision.version().value(),
                    revision.object_type().as_str(),
                    revision.payload()
                ],
            )
            .map_err(sql_store)?;
        Ok(id)
    }
    /// # Parameters
    /// - `id`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Reads an immutable revision by identity.
    fn get(&self, id: RevisionId) -> Result<Option<Revision>, StoreError> {
        let connection = self.connection()?;
        let row = connection
            .query_row(
                "SELECT version,object_type,payload FROM revisions WHERE id=?",
                params![id.as_bytes().as_slice()],
                |row| {
                    Ok((
                        row.get::<_, u16>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(sql_store)?;
        row.map(|(version, kind, payload)| {
            ObjectType::new(kind)
                .map(|object_type| {
                    Revision::new(EncodingVersion::new(version), object_type, payload)
                })
                .map_err(|_| StoreError::Backend("invalid object type".to_owned()))
        })
        .transpose()
    }
    /// # Parameters
    /// - `id`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Checks whether an immutable revision is retained.
    fn contains(&self, id: RevisionId) -> Result<bool, StoreError> {
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT 1 FROM revisions WHERE id=?",
                params![id.as_bytes().as_slice()],
                |_| Ok(()),
            )
            .optional()
            .map_err(sql_store)?
            .is_some())
    }
}
