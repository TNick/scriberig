//! Implements relation-specific immutable snapshot operations.

use super::{Snapshot, SnapshotRelation};
use crate::snapshot_relation::SnapshotRelationChange;

impl Snapshot {
    /// Returns semantic relations in deterministic order.
    ///
    /// # Returns
    /// The snapshot's immutable relation set.
    pub fn relations(&self) -> impl Iterator<Item = &SnapshotRelation> {
        self.relations.iter()
    }

    /// Applies relation changes and returns a persistent successor snapshot.
    ///
    /// - `changes`: Relation additions and removals.
    ///
    /// # Returns
    /// The resulting immutable snapshot.
    #[must_use]
    pub fn apply_relation_changes(
        &self,
        changes: impl IntoIterator<Item = SnapshotRelationChange>,
    ) -> Self {
        self.apply_changes_with_relations(std::iter::empty(), changes)
    }

    /// Computes exact typed relation additions and removals.
    ///
    /// - `other`: Snapshot compared with this snapshot.
    ///
    /// # Returns
    /// Deterministically ordered relation changes transforming this snapshot into `other`.
    #[must_use]
    pub fn diff_relation_changes(&self, other: &Self) -> Vec<SnapshotRelationChange> {
        let before = &self.relations;
        let after = &other.relations;
        before
            .difference(after)
            .cloned()
            .map(SnapshotRelationChange::Remove)
            .chain(
                after
                    .difference(before)
                    .cloned()
                    .map(SnapshotRelationChange::Add),
            )
            .collect()
    }
}
