//! Stores and validates persistent snapshot map structures.

use crate::snapshot_integrity::node_id;
use rusqlite::{OptionalExtension, Transaction, params};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_store::SnapshotError;
/// Verifies that a snapshot's persistent map root is present when referenced.
///
/// - `connection`: SQLite connection containing snapshot roots and nodes.
/// - `snapshot`: Exact snapshot identity to check.
///
/// Unit when the snapshot uses a valid root reference or legacy flat index.
///
/// Returns a backend error for malformed or missing root nodes.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate_snapshot_root(
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
/// - `transaction`: Transaction receiving the snapshot.
/// - `snapshot`: Snapshot to persist.
/// - `crate`: Internal visibility marker.
///
/// Unit after all immutable rows are prepared.
///
/// Returns a SQLite insertion error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn store_in_transaction(
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
/// - `transaction`: Transaction receiving the relation.
/// - `snapshot`: Child snapshot identity.
/// - `parent`: Direct parent snapshot identity.
///
/// Unit after the relation is stored.
///
/// Returns a SQLite insertion error.
/// - `crate`: Argument named `crate`.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
/// - `transaction`: Transaction receiving the node.
/// - `entries`: Sorted entity-to-revision mappings.
/// - `RevisionId`: Revision identity component of each mapping.
/// - `RevisionId`: Revision component of each entry.
///
/// The content identity of the stored node.
///
/// Returns the SQLite insertion error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
