//! Durable revision persistence for the SQLite backend.

use crate::SqliteStore;
use rusqlite::{OptionalExtension, Transaction, params};
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

/// Inserts one immutable revision into an existing transaction.
///
/// # Parameters
/// - `transaction`: Transaction receiving the revision.
/// - `revision`: Immutable revision to retain.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Unit after the idempotent insert is prepared.
///
/// # Errors
/// Returns the SQLite insertion error.
pub(crate) fn insert_in_transaction(
    transaction: &Transaction<'_>,
    revision: &Revision,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT OR IGNORE INTO revisions(id,version,object_type,payload)
         VALUES (?,?,?,?)",
        params![
            revision.id().as_bytes().as_slice(),
            revision.version().value(),
            revision.object_type().as_str(),
            revision.payload()
        ],
    )?;
    Ok(())
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
        drop(connection);
        match self.get(id)? {
            Some(existing) if existing == revision => Ok(id),
            Some(_) => Err(StoreError::IntegrityCollision { id }),
            None => Err(StoreError::Backend("revision was not retained".to_owned())),
        }
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
            let object_type = ObjectType::new(kind)
                .map_err(|_| StoreError::Backend("invalid object type".to_owned()))?;
            Revision::from_parts(id, EncodingVersion::new(version), object_type, payload)
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

#[cfg(test)]
mod tests {
    use super::*;
    use scriberig_core::EncodingError;

    /// Rejects a durable row whose payload no longer matches its identity.
    #[test]
    fn reads_reject_rows_with_mismatched_content_identity() {
        let store = SqliteStore::open_in_memory().expect("open");
        let object_type = ObjectType::new("test.item").expect("type");
        let revision = Revision::new(EncodingVersion::V1, object_type, vec![1]).expect("revision");
        let id = store.put(revision).expect("put");
        store
            .connection()
            .expect("connection")
            .execute(
                "UPDATE revisions SET payload=? WHERE id=?",
                rusqlite::params![vec![2_u8], id.as_bytes().as_slice()],
            )
            .expect("corrupt fixture");

        assert!(matches!(
            store.get(id),
            Err(StoreError::RevisionIdentityMismatch { .. })
        ));
    }

    /// Rejects durable rows using an encoding version absent from this build.
    #[test]
    fn reads_reject_rows_with_unsupported_encoding_version() {
        let store = SqliteStore::open_in_memory().expect("open");
        let object_type = ObjectType::new("test.item").expect("type");
        let revision = Revision::new(EncodingVersion::V1, object_type, vec![1]).expect("revision");
        let id = store.put(revision).expect("put");
        store
            .connection()
            .expect("connection")
            .execute(
                "UPDATE revisions SET version=? WHERE id=?",
                rusqlite::params![2_u16, id.as_bytes().as_slice()],
            )
            .expect("corrupt fixture");

        assert_eq!(
            store.get(id),
            Err(StoreError::InvalidCanonicalEncoding(
                EncodingError::UnsupportedVersion { version: 2 }
            ))
        );
    }
}
