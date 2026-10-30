//! Durable snapshot persistence for the SQLite backend.

use crate::SqliteStore;
use rusqlite::{OptionalExtension, Transaction, params};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_store::{
    SnapshotChange, SnapshotChangeProvenance, SnapshotDiff, SnapshotError, SnapshotRelation,
    SnapshotRelationChange, SnapshotStore,
};

impl SnapshotStore for SqliteStore {
    /// - `snapshot`: Operation input.
    /// - `entity`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// Resolves an entity in a stored snapshot.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `base`: Operation input.
    /// - `changes`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// Applies changes and persists the resulting immutable snapshot.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn apply_changes(
        &self,
        base: SnapshotId,
        changes: &[SnapshotChange],
    ) -> Result<SnapshotId, SnapshotError> {
        let next = self.snapshot(base)?.apply(changes.iter().copied());
        self.store_snapshot_with_parent(&next, base)?;
        Ok(next.id())
    }

    /// Applies typed relation changes and persists the immutable successor.
    ///
    /// - `base`: Existing snapshot identity.
    /// - `changes`: Typed relation additions and removals.
    ///
    /// # Returns
    /// The resulting immutable snapshot identity.
    ///
    /// # Errors
    /// Returns an error when the base is unknown or SQLite rejects the transaction.
    fn apply_relation_changes(
        &self,
        base: SnapshotId,
        changes: &[SnapshotRelationChange],
    ) -> Result<SnapshotId, SnapshotError> {
        let next = self
            .snapshot(base)?
            .apply_relation_changes(changes.iter().cloned());
        self.store_snapshot_with_parent(&next, base)?;
        Ok(next.id())
    }

    /// Applies both entity and relation changes as one SQLite transaction.
    ///
    /// - `base`: Existing snapshot identity.
    /// - `entity_changes`: Entity-to-revision changes.
    /// - `relation_changes`: Typed relation changes.
    ///
    /// # Returns
    /// The resulting immutable snapshot identity.
    ///
    /// # Errors
    /// Returns an error when the base is unknown or SQLite rejects the transaction.
    fn apply_changes_with_relations(
        &self,
        base: SnapshotId,
        entity_changes: &[SnapshotChange],
        relation_changes: &[SnapshotRelationChange],
    ) -> Result<SnapshotId, SnapshotError> {
        let next = self.snapshot(base)?.apply_changes_with_relations(
            entity_changes.iter().copied(),
            relation_changes.iter().cloned(),
        );
        self.store_snapshot_with_parent(&next, base)?;
        Ok(next.id())
    }

    /// Reads typed relations from one exact retained snapshot.
    ///
    /// - `snapshot`: Exact snapshot identity.
    ///
    /// # Returns
    /// Relations in deterministic order.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unavailable or storage is malformed.
    fn relations(&self, snapshot: SnapshotId) -> Result<Vec<SnapshotRelation>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.relations().cloned().collect())
    }

    /// - `snapshot`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// Iterates a stored snapshot in deterministic order.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn iter(&self, snapshot: SnapshotId) -> Result<Vec<(EntityId, RevisionId)>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.iter().collect())
    }
    /// - `from`: Operation input.
    /// - `to`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// Computes exact mapping changes between stored snapshots.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn diff_snapshots(
        &self,
        from: SnapshotId,
        to: SnapshotId,
    ) -> Result<Vec<SnapshotChange>, SnapshotError> {
        Ok(self.snapshot(from)?.diff_changes(&self.snapshot(to)?))
    }

    /// Computes semantic revision transitions from one read of each snapshot.
    ///
    /// - `from`: Starting snapshot identity.
    /// - `to`: Ending snapshot identity.
    ///
    /// Deterministically ordered additions, removals, and revision changes.
    ///
    /// Returns an error when either snapshot is unavailable or unreadable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn diff_snapshot_revisions(
        &self,
        from: SnapshotId,
        to: SnapshotId,
    ) -> Result<Vec<SnapshotDiff>, SnapshotError> {
        Ok(self.snapshot(from)?.diff(&self.snapshot(to)?))
    }

    /// Returns direct immutable predecessors retained for one snapshot.
    ///
    /// - `snapshot`: Snapshot whose direct parents are requested.
    ///
    /// Direct parents in deterministic order.
    ///
    /// Returns an error when the snapshot or parent records cannot be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn parents(&self, snapshot: SnapshotId) -> Result<Vec<SnapshotId>, SnapshotError> {
        if snapshot == scriberig_store::Snapshot::empty().id() {
            return Ok(Vec::new());
        }
        let connection = self
            .connection()
            .map_err(|error| SnapshotError::Backend(format!("{error:?}")))?;
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM snapshots WHERE id=?)",
                params![snapshot.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .map_err(|error| SnapshotError::Backend(error.to_string()))?;
        if !exists {
            return Err(SnapshotError::UnknownSnapshot(snapshot));
        }
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

    /// Reads the durable accepted change associated with an exact result snapshot.
    ///
    /// - `snapshot`: Exact immutable result snapshot.
    ///
    /// The accepted project source-change identity when one produced this snapshot.
    ///
    /// Returns a backend error when the receipt cannot be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn change_provenance(
        &self,
        snapshot: SnapshotId,
    ) -> Result<Option<SnapshotChangeProvenance>, SnapshotError> {
        let connection = self
            .connection()
            .map_err(|error| SnapshotError::Backend(format!("{error:?}")))?;
        let provenance = connection
            .query_row(
                "SELECT effects.candidate_id, candidates.semantic_operation
                 FROM project_effects AS effects
                 JOIN project_candidates AS candidates ON candidates.id=effects.candidate_id
                 WHERE effects.successor_snapshot=? AND effects.state='applied'
                 ORDER BY effects.candidate_id LIMIT 1",
                params![snapshot.as_bytes().as_slice()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(|error| SnapshotError::Backend(error.to_string()))?;
        Ok(
            provenance.map(|(change_id, semantic_operation)| SnapshotChangeProvenance {
                change_id,
                semantic_operation,
            }),
        )
    }
}

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
pub(crate) fn store_in_transaction(
    transaction: &Transaction<'_>,
    snapshot: &scriberig_store::Snapshot,
) -> rusqlite::Result<()> {
    let entries: Vec<_> = snapshot.iter().collect();
    let root = crate::snapshot_node_store::store_node(transaction, &entries)?;
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
    crate::snapshot_relations::store_relations_in_transaction(transaction, snapshot)?;
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
