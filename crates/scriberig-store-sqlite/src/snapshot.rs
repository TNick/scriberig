//! Durable snapshot persistence for the SQLite backend.

use crate::SqliteStore;
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
