//! Defines backend-neutral operations over retained immutable snapshots.

use scriberig_core::{EntityId, RevisionId, SnapshotId};
use std::collections::BTreeSet;

use crate::{SnapshotChange, SnapshotError};

/// Defines backend-neutral snapshot operations.
pub trait SnapshotStore {
    /// Resolves an entity in a stored snapshot.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot identity.
    /// - `entity`: Entity identity to resolve.
    ///
    /// # Returns
    /// The revision mapped to the entity, when present.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unavailable.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, SnapshotError>;

    /// Applies storage changes and returns the resulting snapshot.
    ///
    /// # Parameters
    /// - `base`: Existing snapshot identity.
    /// - `changes`: Exact entity mapping changes to apply.
    ///
    /// # Returns
    /// The immutable resulting snapshot identity.
    ///
    /// # Errors
    /// Returns an error when the base snapshot is unavailable.
    fn apply_changes(
        &self,
        base: SnapshotId,
        changes: &[SnapshotChange],
    ) -> Result<SnapshotId, SnapshotError>;

    /// Iterates a stored snapshot deterministically.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot identity.
    ///
    /// # Returns
    /// Entity and revision mappings in ascending entity order.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unavailable.
    #[allow(clippy::iter_not_returning_iterator)]
    fn iter(&self, snapshot: SnapshotId) -> Result<Vec<(EntityId, RevisionId)>, SnapshotError>;

    /// Computes exact mapping changes between stored snapshots.
    ///
    /// # Parameters
    /// - `from`: Starting snapshot identity.
    /// - `to`: Ending snapshot identity.
    ///
    /// # Returns
    /// Deterministically ordered storage changes.
    ///
    /// # Errors
    /// Returns an error when either snapshot is unavailable.
    fn diff_snapshots(
        &self,
        from: SnapshotId,
        to: SnapshotId,
    ) -> Result<Vec<SnapshotChange>, SnapshotError>;

    /// Returns the direct immutable predecessors of a snapshot.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot whose parents are requested.
    ///
    /// # Returns
    /// Deterministically ordered direct parent snapshots.
    ///
    /// # Errors
    /// Returns an error when the snapshot is unavailable.
    fn parents(&self, snapshot: SnapshotId) -> Result<Vec<SnapshotId>, SnapshotError>;

    /// Walks immutable snapshot ancestry without exposing backend details.
    ///
    /// # Parameters
    /// - `snapshot`: Exact snapshot at which to begin the walk.
    ///
    /// # Returns
    /// Ancestors in deterministic breadth-first order, excluding `snapshot`.
    ///
    /// # Errors
    /// Returns an error when the starting snapshot or an ancestor is unavailable.
    fn ancestors(&self, snapshot: SnapshotId) -> Result<Vec<SnapshotId>, SnapshotError> {
        let mut visited = BTreeSet::new();
        let mut pending = self.parents(snapshot)?;
        let mut result = Vec::new();
        while let Some(candidate) = pending.pop() {
            if !visited.insert(candidate) {
                continue;
            }
            result.push(candidate);
            pending.extend(self.parents(candidate)?);
        }
        result.sort();
        Ok(result)
    }
}
