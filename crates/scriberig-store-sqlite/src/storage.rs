//! Implements SQLite snapshot loading and transactional persistence helpers.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension};
use scriberig_core::SnapshotId;
use scriberig_store::{Snapshot, SnapshotError, StoreError};

use crate::{SqliteStore, recovery, snapshot_integrity};

impl SqliteStore {
    /// Returns a locked SQLite connection for backend operations.
    ///
    /// # Parameters
    /// - `self`: Store whose connection is requested.
    /// - `crate`: Internal visibility marker.
    ///
    /// # Returns
    /// The locked SQLite connection.
    ///
    /// # Errors
    /// Returns a lock-poisoning error.
    pub(crate) fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, StoreError> {
        self.connection.lock().map_err(|_| StoreError::LockPoisoned)
    }

    /// Loads and validates one immutable snapshot.
    ///
    /// # Parameters
    /// - `self`: Store containing the snapshot.
    /// - `id`: Snapshot identity to load.
    /// - `crate`: Internal visibility marker.
    ///
    /// # Returns
    /// The validated immutable snapshot.
    ///
    /// # Errors
    /// Returns an unknown-snapshot or backend integrity error.
    pub(crate) fn snapshot(&self, id: SnapshotId) -> Result<Snapshot, SnapshotError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SnapshotError::Backend("lock poisoned".to_owned()))?;
        if id == Snapshot::empty().id() {
            return Ok(Snapshot::empty());
        }
        let root: Option<Vec<u8>> = connection
            .query_row(
                "SELECT node_id FROM snapshot_node_roots WHERE snapshot_id=?",
                rusqlite::params![id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_snapshot)?;
        let Some(root) = root else {
            let exists: bool = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM snapshots WHERE id=?)",
                    rusqlite::params![id.as_bytes().as_slice()],
                    |row| row.get(0),
                )
                .map_err(sql_snapshot)?;
            if !exists {
                return Err(SnapshotError::UnknownSnapshot(id));
            }
            let mut entries = BTreeMap::new();
            let mut statement = connection
                .prepare(
                    "SELECT entity, revision FROM snapshot_entries
                     WHERE snapshot_id=? ORDER BY entity",
                )
                .map_err(sql_snapshot)?;
            let rows = statement
                .query_map(rusqlite::params![id.as_bytes().as_slice()], |row| {
                    let entity: Vec<u8> = row.get(0)?;
                    let revision: Vec<u8> = row.get(1)?;
                    Ok((entity, revision))
                })
                .map_err(sql_snapshot)?;
            for row in rows {
                let (entity, revision) = row.map_err(sql_snapshot)?;
                if entity.len() != 16 || revision.len() != 32 {
                    return Err(SnapshotError::Backend(
                        "invalid stored identifier length".to_owned(),
                    ));
                }
                let mut entity_bytes = [0; 16];
                entity_bytes.copy_from_slice(&entity);
                let mut revision_bytes = [0; 32];
                revision_bytes.copy_from_slice(&revision);
                entries.insert(
                    scriberig_core::EntityId::from_bytes(entity_bytes),
                    scriberig_core::RevisionId::new(revision_bytes),
                );
            }
            let snapshot = Snapshot::from_entries(entries);
            return if snapshot.id() == id {
                Ok(snapshot)
            } else {
                Err(SnapshotError::Backend(
                    "snapshot identity mismatch".to_owned(),
                ))
            };
        };
        let mut entries = BTreeMap::new();
        snapshot_integrity::load_node(&connection, &root, &mut entries).map_err(sql_snapshot)?;
        let snapshot = Snapshot::from_entries(entries);
        if snapshot.id() != id {
            return Err(SnapshotError::Backend(
                "snapshot identity mismatch".to_owned(),
            ));
        }
        Ok(snapshot)
    }

    /// Stores a snapshot and its direct predecessor in one transaction.
    ///
    /// # Parameters
    /// - `snapshot`: Immutable snapshot to persist.
    /// - `parent`: Direct predecessor of the snapshot.
    ///
    /// # Returns
    /// Unit after the transaction commits.
    ///
    /// # Errors
    /// Returns an error when SQLite rejects the transaction.
    /// - `crate`: Argument named `crate`.
    pub(crate) fn store_snapshot_with_parent(
        &self,
        snapshot: &Snapshot,
        parent: SnapshotId,
    ) -> Result<(), SnapshotError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| SnapshotError::Backend("lock poisoned".to_owned()))?;
        recovery::transaction(&mut connection, |transaction| {
            Self::store_snapshot_in_transaction(transaction, snapshot)?;
            crate::snapshot::store_parent_in_transaction(transaction, snapshot.id(), parent)
        })
        .map_err(sql_snapshot)
    }
}

/// Converts a SQLite error to a snapshot backend error.
///
/// # Parameters
/// - `error`: SQLite error.
///
/// # Returns
/// A typed snapshot backend error.
fn sql_snapshot(error: rusqlite::Error) -> SnapshotError {
    SnapshotError::Backend(error.to_string())
}
