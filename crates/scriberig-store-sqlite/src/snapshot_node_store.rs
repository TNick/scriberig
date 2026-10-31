//! Stores content-addressed immutable snapshot mapping nodes.

use rusqlite::{Transaction, params};
use scriberig_core::{EntityId, RevisionId};

use crate::snapshot_integrity::node_id;

/// Stores a balanced persistent node tree and returns its content identity.
///
/// - `transaction`: Transaction receiving the node rows.
/// - `entries`: Sorted entity-to-revision mappings.
///
/// # Returns
/// The content identity of the stored node.
/// # Errors
/// Returns an error when SQLite rejects a node row.
pub(crate) fn store_node(
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
