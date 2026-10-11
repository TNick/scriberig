//! Durable snapshot persistence for the SQLite backend.

use crate::SqliteStore;
use crate::snapshot_integrity::node_id;
use rusqlite::{Transaction, params};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_store::{SnapshotChange, SnapshotError, SnapshotStore};

impl SnapshotStore for SqliteStore {
    /// # Parameters
    /// - `snapshot`: Operation input.
    /// - `entity`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Resolves an entity in a stored snapshot.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.resolve(entity))
    }
    /// # Parameters
    /// - `base`: Operation input.
    /// - `changes`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Applies changes and persists the resulting immutable snapshot.
    fn apply_changes(
        &self,
        base: SnapshotId,
        changes: &[SnapshotChange],
    ) -> Result<SnapshotId, SnapshotError> {
        let next = self.snapshot(base)?.apply(changes.iter().copied());
        self.store_snapshot(&next)?;
        Ok(next.id())
    }
    /// # Parameters
    /// - `snapshot`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Iterates a stored snapshot in deterministic order.
    fn iter(&self, snapshot: SnapshotId) -> Result<Vec<(EntityId, RevisionId)>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.iter().collect())
    }
    /// # Parameters
    /// - `from`: Operation input.
    /// - `to`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Computes exact mapping changes between stored snapshots.
    fn diff_snapshots(
        &self,
        from: SnapshotId,
        to: SnapshotId,
    ) -> Result<Vec<SnapshotChange>, SnapshotError> {
        Ok(self.snapshot(from)?.diff(&self.snapshot(to)?))
    }
}

/// Persists a snapshot and its content-addressed binary map nodes.
///
/// # Parameters
/// - `transaction`: Transaction receiving the snapshot.
/// - `snapshot`: Snapshot to persist.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Unit after all immutable rows are prepared.
///
/// # Errors
/// Returns a SQLite insertion error.
pub(crate) fn store_in_transaction(
    transaction: &Transaction<'_>,
    snapshot: &scriberig_store::Snapshot,
) -> rusqlite::Result<()> {
    let entries: Vec<_> = snapshot.iter().collect();
    let root = store_node(transaction, &entries)?;
    transaction.execute(
        "INSERT OR IGNORE INTO snapshots(id) VALUES (?)",
        params![snapshot.id().as_bytes().as_slice()],
    )?;
    transaction.execute(
        "INSERT OR IGNORE INTO snapshot_node_roots(snapshot_id,node_id) VALUES (?,?)",
        params![snapshot.id().as_bytes().as_slice(), root.as_slice()],
    )?;
    for (entity, revision) in entries {
        transaction.execute(
            "INSERT OR IGNORE INTO snapshot_entries(snapshot_id,entity,revision)
             VALUES (?,?,?)",
            params![
                snapshot.id().as_bytes().as_slice(),
                entity.as_bytes().as_slice(),
                revision.as_bytes().as_slice()
            ],
        )?;
    }
    Ok(())
}

/// Stores a balanced persistent node tree and returns its content identity.
///
/// # Parameters
/// - `transaction`: Transaction receiving the node.
/// - `entries`: Sorted entity-to-revision mappings.
/// - `RevisionId`: Revision identity component of each mapping.
/// - `RevisionId`: Revision component of each entry.
///
/// # Returns
/// The content identity of the stored node.
///
/// # Errors
/// Returns the SQLite insertion error.
fn store_node(
    transaction: &Transaction<'_>,
    entries: &[(EntityId, RevisionId)],
) -> rusqlite::Result<Vec<u8>> {
    if entries.is_empty() {
        let id = node_id(0, None, None, None, None);
        transaction.execute(
            "INSERT OR IGNORE INTO snapshot_nodes
             (id,kind,entity,revision,left_child,right_child,item_count)
             VALUES (?,?,?,?,?,?,?)",
            params![
                id,
                0_i64,
                Option::<Vec<u8>>::None,
                Option::<Vec<u8>>::None,
                Option::<Vec<u8>>::None,
                Option::<Vec<u8>>::None,
                0_i64
            ],
        )?;
        return Ok(id);
    }
    if entries.len() == 1 {
        let (entity, revision) = entries[0];
        let entity = entity.as_bytes().to_vec();
        let revision = revision.as_bytes().to_vec();
        let id = node_id(1, Some(&entity), Some(&revision), None, None);
        transaction.execute(
            "INSERT OR IGNORE INTO snapshot_nodes
             (id,kind,entity,revision,left_child,right_child,item_count)
             VALUES (?,?,?,?,?,?,?)",
            params![
                id,
                1_i64,
                entity,
                revision,
                Option::<Vec<u8>>::None,
                Option::<Vec<u8>>::None,
                1_i64
            ],
        )?;
        return Ok(id);
    }
    let middle = entries.len() / 2;
    let left = store_node(transaction, &entries[..middle])?;
    let right = store_node(transaction, &entries[middle..])?;
    let id = node_id(2, None, None, Some(&left), Some(&right));
    transaction.execute(
        "INSERT OR IGNORE INTO snapshot_nodes
         (id,kind,entity,revision,left_child,right_child,item_count)
         VALUES (?,?,?,?,?,?,?)",
        params![
            id,
            2_i64,
            Option::<Vec<u8>>::None,
            Option::<Vec<u8>>::None,
            left,
            right,
            i64::try_from(entries.len()).map_err(|_| rusqlite::Error::InvalidQuery)?
        ],
    )?;
    Ok(id)
}
