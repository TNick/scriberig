//! Durable snapshot persistence for the SQLite backend.

use crate::SqliteStore;
#[path = "snapshot_helpers.rs"]
mod helpers;
pub(crate) use helpers::store_parent_in_transaction;
use helpers::{store_in_transaction, validate_snapshot_root};
use rusqlite::{OptionalExtension, Transaction, params};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_store::{
    SnapshotChange, SnapshotChangeProvenance, SnapshotDiff, SnapshotError, SnapshotStore,
};

impl SqliteStore {
    /// Stores a snapshot and its persistent map root in an open transaction.
    ///
    /// # Parameters
    /// - `transaction`: Transaction receiving the immutable records.
    /// - `snapshot`: Snapshot to persist.
    ///
    /// # Returns
    /// Unit after the snapshot records are prepared.
    ///
    /// # Errors
    /// Returns a SQLite insertion error when the records cannot be stored.
    pub(crate) fn store_snapshot_in_transaction(
        transaction: &Transaction<'_>,
        snapshot: &scriberig_store::Snapshot,
    ) -> rusqlite::Result<()> {
        store_in_transaction(transaction, snapshot)
    }
}

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
        let change_id = connection
            .query_row(
                "SELECT candidate_id FROM project_effects
                 WHERE successor_snapshot=? AND state='applied' ORDER BY candidate_id LIMIT 1",
                params![snapshot.as_bytes().as_slice()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| SnapshotError::Backend(error.to_string()))?;
        Ok(change_id.map(|change_id| SnapshotChangeProvenance { change_id }))
    }
}
