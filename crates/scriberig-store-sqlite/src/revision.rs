//! Durable revision persistence for the SQLite backend.

use crate::SqliteStore;
use rusqlite::{OptionalExtension, Transaction, params};
use scriberig_core::{EncodingVersion, ObjectType, RevisionId};
use scriberig_store::{Revision, RevisionStore, StoreError};
use std::collections::BTreeMap;

/// Converts a SQLite error to a revision backend error.
///
/// - `error`: SQLite error.
///
/// A typed revision backend error.
/// # Returns
/// The operation result.
fn sql_store(error: rusqlite::Error) -> StoreError {
    StoreError::Backend(error.to_string())
}

/// Inserts one immutable revision into an existing transaction.
///
/// - `transaction`: Transaction receiving the revision.
/// - `revision`: Immutable revision to retain.
/// - `crate`: Internal visibility marker.
///
/// Unit after the idempotent insert is prepared.
///
/// Returns the SQLite insertion error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
    /// - `revision`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// Inserts an immutable revision idempotently.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `id`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// Reads an immutable revision by identity.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// Reads immutable revisions in bounded payload queries.
    ///
    /// # Parameters
    /// - `ids`: Immutable revision identities to load.
    ///
    /// # Returns
    /// Retained revisions keyed by identity; missing IDs are omitted.
    ///
    /// # Errors
    /// Returns a storage, encoding, or integrity error.
    fn get_many(&self, ids: &[RevisionId]) -> Result<BTreeMap<RevisionId, Revision>, StoreError> {
        let mut revisions = BTreeMap::new();
        let connection = self.connection()?;
        for chunk in ids.chunks(400) {
            if chunk.is_empty() {
                continue;
            }
            let placeholders = (1..=chunk.len())
                .map(|index| format!("?{index}"))
                .collect::<Vec<_>>()
                .join(",");
            let query = format!(
                "SELECT id,version,object_type,payload FROM revisions WHERE id IN ({placeholders})"
            );
            let values = chunk
                .iter()
                .map(|id| id.as_bytes().to_vec())
                .collect::<Vec<_>>();
            let mut statement = connection.prepare(&query).map_err(sql_store)?;
            let rows = statement
                .query_map(rusqlite::params_from_iter(values.iter()), |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, u16>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                })
                .map_err(sql_store)?;
            for row in rows {
                let (bytes, version, kind, payload) = row.map_err(sql_store)?;
                let bytes: [u8; 32] = bytes
                    .try_into()
                    .map_err(|_| StoreError::Backend("invalid revision identity".to_owned()))?;
                let id = RevisionId::from_bytes(bytes);
                let object_type = ObjectType::new(kind)
                    .map_err(|_| StoreError::Backend("invalid object type".to_owned()))?;
                let revision =
                    Revision::from_parts(id, EncodingVersion::new(version), object_type, payload)?;
                revisions.insert(id, revision);
            }
        }
        Ok(revisions)
    }
    /// Reads revision type metadata without loading its content payload.
    ///
    /// # Parameters
    /// - `id`: Immutable revision identity.
    ///
    /// # Returns
    /// The declared object type, or `None` when the revision is absent.
    ///
    /// # Errors
    /// Returns a backend error when SQLite or type validation fails.
    fn get_object_type(
        &self,
        id: RevisionId,
    ) -> Result<Option<scriberig_core::TypeId>, StoreError> {
        let connection = self.connection()?;
        let kind = connection
            .query_row(
                "SELECT object_type FROM revisions WHERE id=?",
                params![id.as_bytes().as_slice()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_store)?;
        kind.map(|value| {
            ObjectType::new(value)
                .map_err(|_| StoreError::Backend("invalid object type".to_owned()))
        })
        .transpose()
    }
    /// Reads declared revision types in bounded metadata-only queries.
    ///
    /// # Parameters
    /// - `ids`: Immutable revision identities to inspect.
    ///
    /// # Returns
    /// Types for retained revisions, keyed by revision identity.
    ///
    /// # Errors
    /// Returns a backend error for invalid metadata or SQLite failures.
    fn get_object_types(
        &self,
        ids: &[RevisionId],
    ) -> Result<BTreeMap<RevisionId, scriberig_core::TypeId>, StoreError> {
        let mut types = BTreeMap::new();
        let connection = self.connection()?;
        for chunk in ids.chunks(400) {
            if chunk.is_empty() {
                continue;
            }
            let placeholders = (1..=chunk.len())
                .map(|index| format!("?{index}"))
                .collect::<Vec<_>>()
                .join(",");
            let query =
                format!("SELECT id,object_type FROM revisions WHERE id IN ({placeholders})");
            let values = chunk
                .iter()
                .map(|id| id.as_bytes().to_vec())
                .collect::<Vec<_>>();
            let mut statement = connection.prepare(&query).map_err(sql_store)?;
            let rows = statement
                .query_map(rusqlite::params_from_iter(values.iter()), |row| {
                    Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(sql_store)?;
            for row in rows {
                let (bytes, kind) = row.map_err(sql_store)?;
                let bytes: [u8; 32] = bytes
                    .try_into()
                    .map_err(|_| StoreError::Backend("invalid revision identity".to_owned()))?;
                let id = RevisionId::from_bytes(bytes);
                let object_type = ObjectType::new(kind)
                    .map_err(|_| StoreError::Backend("invalid object type".to_owned()))?;
                types.insert(id, object_type);
            }
        }
        Ok(types)
    }
    /// - `id`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// Checks whether an immutable revision is retained.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
