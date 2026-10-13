//! Durable snapshot persistence for the SQLite backend.

use crate::SqliteStore;
use crate::snapshot_integrity::node_id;
use rusqlite::{OptionalExtension, Transaction, params};
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
        let connection = self
            .connection()
            .map_err(|error| SnapshotError::Backend(format!("{error:?}")))?;
        if snapshot == scriberig_store::Snapshot::empty().id() {
            return Ok(None);
        }
        validate_snapshot_root(&connection, snapshot)?;
        let revision: Option<Vec<u8>> = connection
            .query_row(
                "SELECT revision FROM snapshot_entries WHERE snapshot_id=? AND entity=?",
                params![snapshot.as_bytes().as_slice(), entity.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| SnapshotError::Backend(format!("{error:?}")))?;
        if let Some(revision) = revision {
            let bytes: [u8; 32] = revision.try_into().map_err(|_| {
                SnapshotError::Backend("invalid stored revision identifier length".to_owned())
            })?;
            return Ok(Some(RevisionId::new(bytes)));
        }
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM snapshots WHERE id=?)",
                params![snapshot.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .map_err(|error| SnapshotError::Backend(format!("{error:?}")))?;
        if exists {
            Ok(None)
        } else {
            Err(SnapshotError::UnknownSnapshot(snapshot))
        }
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
        self.store_snapshot_with_parent(&next, base)?;
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
        Ok(self.snapshot(from)?.diff_changes(&self.snapshot(to)?))
    }

    /// Returns direct immutable predecessors retained for one snapshot.
    ///
    /// # Parameters
    /// - `snapshot`: Snapshot whose direct parents are requested.
    ///
    /// # Returns
    /// Direct parents in deterministic order.
    ///
    /// # Errors
    /// Returns an error when the snapshot or parent records cannot be read.
    fn parents(&self, snapshot: SnapshotId) -> Result<Vec<SnapshotId>, SnapshotError> {
        let _ = self.snapshot(snapshot)?;
        let connection = self
            .connection()
            .map_err(|error| SnapshotError::Backend(format!("{error:?}")))?;
        let mut statement = connection
            .prepare(
                "SELECT parent_id FROM snapshot_parents
                 WHERE snapshot_id=? ORDER BY parent_id",
            )
            .map_err(|error| SnapshotError::Backend(error.to_string()))?;
        let rows = statement
            .query_map(params![snapshot.as_bytes().as_slice()], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                if bytes.len() != 32 {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                let mut value = [0; 32];
                value.copy_from_slice(&bytes);
                Ok(SnapshotId::from_bytes(value))
            })
            .map_err(|error| SnapshotError::Backend(error.to_string()))?;
        rows.map(|row| row.map_err(|error| SnapshotError::Backend(error.to_string())))
            .collect()
    }
}

/// Verifies that a snapshot's persistent map root is present when referenced.
///
/// # Parameters
/// - `connection`: SQLite connection containing snapshot roots and nodes.
/// - `snapshot`: Exact snapshot identity to check.
///
/// # Returns
/// Unit when the snapshot uses a valid root reference or legacy flat index.
///
/// # Errors
/// Returns a backend error for malformed or missing root nodes.
fn validate_snapshot_root(
    connection: &rusqlite::Connection,
    snapshot: SnapshotId,
) -> Result<(), SnapshotError> {
    let root: Option<Vec<u8>> = connection
        .query_row(
            "SELECT node_id FROM snapshot_node_roots WHERE snapshot_id=?",
            params![snapshot.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| SnapshotError::Backend(error.to_string()))?;
    let Some(root) = root else {
        return Ok(());
    };
    if root.len() != 32 {
        return Err(SnapshotError::Backend(
            "invalid stored snapshot node identity length".to_owned(),
        ));
    }
    let present: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM snapshot_nodes WHERE id=?)",
            params![root],
            |row| row.get(0),
        )
        .map_err(|error| SnapshotError::Backend(error.to_string()))?;
    if present {
        Ok(())
    } else {
        Err(SnapshotError::Backend(
            "missing snapshot root node".to_owned(),
        ))
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

/// Stores one direct immutable snapshot-parent relation.
///
/// # Parameters
/// - `transaction`: Transaction receiving the relation.
/// - `snapshot`: Child snapshot identity.
/// - `parent`: Direct parent snapshot identity.
///
/// # Returns
/// Unit after the relation is stored.
///
/// # Errors
/// Returns a SQLite insertion error.
/// - `crate`: Argument named `crate`.
pub(crate) fn store_parent_in_transaction(
    transaction: &Transaction<'_>,
    snapshot: SnapshotId,
    parent: SnapshotId,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT OR IGNORE INTO snapshot_parents(snapshot_id,parent_id) VALUES (?,?)",
        params![snapshot.as_bytes().as_slice(), parent.as_bytes().as_slice()],
    )?;
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
