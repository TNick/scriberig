//! Implements immutable snapshot comparison helpers.

use super::{Snapshot, SnapshotChange, SnapshotDiff};
use std::collections::BTreeMap;

impl Snapshot {
    /// Computes explicit semantic differences in ascending entity order.
    ///
    /// # Parameters
    /// - `other`: Snapshot to compare with this snapshot.
    ///
    /// # Returns
    /// Semantic changes between the snapshots.
    #[must_use]
    pub fn diff(&self, other: &Self) -> Vec<SnapshotDiff> {
        self.diff_semantic(other)
    }

    /// Computes storage-level changes suitable for applying to another snapshot.
    ///
    /// # Parameters
    /// - `other`: Snapshot to compare with this snapshot.
    ///
    /// # Returns
    /// Storage changes that transform this snapshot into the other snapshot.
    #[must_use]
    pub fn diff_changes(&self, other: &Self) -> Vec<SnapshotChange> {
        self.diff_semantic(other)
            .into_iter()
            .map(|change| match change {
                SnapshotDiff::Added { entity, revision }
                | SnapshotDiff::Changed {
                    entity,
                    to: revision,
                    ..
                } => SnapshotChange::Set(entity, revision),
                SnapshotDiff::Removed { entity, .. } => SnapshotChange::Remove(entity),
            })
            .collect()
    }

    /// Computes explicit semantic differences in ascending entity order.
    ///
    /// # Parameters
    /// - `other`: Snapshot to compare with this snapshot.
    ///
    /// # Returns
    /// Semantic changes between the snapshots.
    #[must_use]
    pub fn diff_semantic(&self, other: &Self) -> Vec<SnapshotDiff> {
        let left: BTreeMap<_, _> = self.iter().collect();
        let right: BTreeMap<_, _> = other.iter().collect();
        left.keys()
            .chain(right.keys())
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter_map(|entity| match (left.get(&entity), right.get(&entity)) {
                (None, Some(revision)) => Some(SnapshotDiff::Added {
                    entity,
                    revision: *revision,
                }),
                (Some(revision), None) => Some(SnapshotDiff::Removed {
                    entity,
                    revision: *revision,
                }),
                (Some(from), Some(to)) if from != to => Some(SnapshotDiff::Changed {
                    entity,
                    from: *from,
                    to: *to,
                }),
                _ => None,
            })
            .collect()
    }
}
