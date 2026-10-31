//! Persists typed relation state for immutable SQLite snapshots.

use rusqlite::{Connection, Transaction, params};
use scriberig_core::SnapshotId;
use scriberig_store::{Snapshot, SnapshotError, SnapshotRelation, SnapshotRelationChange};

/// Inserts relation rows associated with one snapshot in a transaction.
///
/// - `transaction`: SQLite transaction receiving the rows.
/// - `snapshot`: Immutable snapshot whose relations are persisted.
///
/// # Returns
/// Unit after every relation is inserted or already present.
///
/// Returns the SQLite error for a rejected row.
/// # Errors
/// Returns an error when SQLite rejects a relation row.
pub(crate) fn store_relations_in_transaction(
    transaction: &Transaction<'_>,
    snapshot: &scriberig_store::Snapshot,
) -> rusqlite::Result<()> {
    for relation in snapshot.relations() {
        transaction.execute(
            "INSERT OR IGNORE INTO snapshot_relations(snapshot_id,source_entity,relation_type,target_entity)
             VALUES (?,?,?,?)",
            params![
                snapshot.id().as_bytes().as_slice(),
                relation.from.as_bytes().as_slice(),
                relation.relation_type.as_str(),
                relation.to.as_bytes().as_slice()
            ],
        )?;
    }
    Ok(())
}

/// Attaches durable typed relations to one decoded snapshot mapping.
///
/// - `connection`: Database containing snapshot relation rows.
/// - `id`: Exact snapshot identity.
/// - `snapshot`: Decoded entity mapping.
///
/// # Returns
/// The snapshot including its canonical relation state.
///
/// # Errors
/// Returns an error for malformed relation data or failed SQLite reads.
pub(crate) fn attach_relations(
    connection: &Connection,
    id: SnapshotId,
    snapshot: Snapshot,
) -> Result<Snapshot, SnapshotError> {
    let mut statement = connection
        .prepare("SELECT source_entity, relation_type, target_entity FROM snapshot_relations WHERE snapshot_id=? ORDER BY source_entity, relation_type, target_entity")
        .map_err(relation_snapshot_error)?;
    let rows = statement
        .query_map(params![id.as_bytes().as_slice()], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(relation_snapshot_error)?;
    let mut changes = Vec::new();
    for row in rows {
        let (from, relation_type, to) = row.map_err(relation_snapshot_error)?;
        if from.len() != 16 || to.len() != 16 {
            return Err(SnapshotError::Backend(
                "invalid stored relation entity identifier length".to_owned(),
            ));
        }
        let mut from_bytes = [0; 16];
        from_bytes.copy_from_slice(&from);
        let mut to_bytes = [0; 16];
        to_bytes.copy_from_slice(&to);
        let relation_type = scriberig_core::TypeId::new(relation_type)
            .map_err(|error| SnapshotError::Backend(error.to_string()))?;
        changes.push(SnapshotRelationChange::Add(SnapshotRelation::new(
            scriberig_core::EntityId::from_bytes(from_bytes),
            relation_type,
            scriberig_core::EntityId::from_bytes(to_bytes),
        )));
    }
    Ok(snapshot.apply_relation_changes(changes))
}

/// Converts an SQLite query failure into a snapshot backend error.
///
/// - `error`: Failed SQLite operation.
///
/// # Returns
/// The corresponding backend error.
fn relation_snapshot_error(error: rusqlite::Error) -> SnapshotError {
    SnapshotError::Backend(error.to_string())
}
