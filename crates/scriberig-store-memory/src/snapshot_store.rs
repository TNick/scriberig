//! Implements retained snapshot operations for the in-memory backend.

use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_store::{
    SnapshotChange, SnapshotError, SnapshotRelation, SnapshotRelationChange, SnapshotStore,
};

use crate::MemoryStore;

impl SnapshotStore for MemoryStore {
    /// - `snapshot`: Relevant operation input.
    /// - `entity`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Resolves an entity from a retained snapshot.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.resolve(entity))
    }

    /// - `base`: Relevant operation input.
    /// - `changes`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Applies changes and retains the resulting immutable snapshot.
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
        let id = next.id();
        self.snapshots
            .write()
            .map_err(|_| SnapshotError::UnknownSnapshot(id))?
            .entry(id)
            .or_insert(next);
        if id != base {
            self.parents
                .write()
                .map_err(|_| SnapshotError::UnknownSnapshot(id))?
                .entry(id)
                .or_default()
                .insert(base);
        }
        Ok(id)
    }

    /// Applies relation changes and retains the resulting immutable snapshot.
    ///
    /// - `base`: Snapshot receiving the changes.
    /// - `changes`: Typed relation additions and removals.
    ///
    /// # Returns
    /// The resulting snapshot identity.
    ///
    /// # Errors
    /// Returns an error when the snapshot store lock is poisoned or the base is unknown.
    fn apply_relation_changes(
        &self,
        base: SnapshotId,
        changes: &[SnapshotRelationChange],
    ) -> Result<SnapshotId, SnapshotError> {
        let next = self
            .snapshot(base)?
            .apply_relation_changes(changes.iter().cloned());
        let id = next.id();
        self.snapshots
            .write()
            .map_err(|_| SnapshotError::UnknownSnapshot(id))?
            .entry(id)
            .or_insert(next);
        if id != base {
            self.parents
                .write()
                .map_err(|_| SnapshotError::UnknownSnapshot(id))?
                .entry(id)
                .or_default()
                .insert(base);
        }
        Ok(id)
    }

    /// Applies both entity and relation changes as one immutable transition.
    ///
    /// - `base`: Snapshot receiving the changes.
    /// - `entity_changes`: Entity-to-revision changes.
    /// - `relation_changes`: Typed relation changes.
    ///
    /// # Returns
    /// The resulting snapshot identity.
    ///
    /// # Errors
    /// Returns an error when the base snapshot is unavailable or a store lock is poisoned.
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
        let id = next.id();
        self.snapshots
            .write()
            .map_err(|_| SnapshotError::UnknownSnapshot(id))?
            .entry(id)
            .or_insert(next);
        if id != base {
            self.parents
                .write()
                .map_err(|_| SnapshotError::UnknownSnapshot(id))?
                .entry(id)
                .or_default()
                .insert(base);
        }
        Ok(id)
    }

    /// Reads typed relations from one retained immutable snapshot.
    ///
    /// - `snapshot`: Exact snapshot identity.
    ///
    /// # Returns
    /// Relations in deterministic order.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unavailable.
    fn relations(&self, snapshot: SnapshotId) -> Result<Vec<SnapshotRelation>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.relations().cloned().collect())
    }

    /// - `snapshot`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Iterates a retained snapshot in entity order.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn iter(&self, snapshot: SnapshotId) -> Result<Vec<(EntityId, RevisionId)>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.iter().collect())
    }

    /// - `from`: Relevant operation input.
    /// - `to`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Computes deterministic changes between retained snapshots.
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

    /// Returns direct immutable predecessors retained for one snapshot.
    ///
    /// - `snapshot`: Snapshot whose direct parents are requested.
    ///
    /// Direct parents in deterministic order.
    ///
    /// Returns an error when the snapshot is unavailable or the store lock is poisoned.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn parents(&self, snapshot: SnapshotId) -> Result<Vec<SnapshotId>, SnapshotError> {
        let _ = self.snapshot(snapshot)?;
        Ok(self
            .parents
            .read()
            .map_err(|_| SnapshotError::UnknownSnapshot(snapshot))?
            .get(&snapshot)
            .map(|parents| parents.iter().copied().collect())
            .unwrap_or_default())
    }
}
