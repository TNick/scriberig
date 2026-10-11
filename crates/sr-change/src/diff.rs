//! Defines deterministic, backend-independent comparisons between snapshots.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::{EntityId, SnapshotId};
use scriberig_store::{SnapshotError, SnapshotStore};

use crate::EntityDelta;

/// The complete semantic difference between two exact snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDiff {
    /// Snapshot on the left side of the comparison.
    pub left: SnapshotId,
    /// Snapshot on the right side of the comparison.
    pub right: SnapshotId,
    /// Changed entities in ascending canonical entity-id order.
    pub deltas: Vec<EntityDelta>,
}

impl SemanticDiff {
    /// Creates a deterministic snapshot diff from entity deltas.
    ///
    /// # Parameters
    /// - `left`: Snapshot on the left side.
    /// - `right`: Snapshot on the right side.
    /// - `deltas`: Changed entities.
    ///
    /// # Returns
    /// A diff with stable entity ordering.
    #[must_use]
    pub fn new(left: SnapshotId, right: SnapshotId, mut deltas: Vec<EntityDelta>) -> Self {
        deltas.sort_by_key(|delta| delta.entity);
        Self {
            left,
            right,
            deltas,
        }
    }

    /// Produces compact counts without rendering domain-specific data.
    ///
    /// # Returns
    /// Counts of introduced, removed, modified, and total changed entities.
    #[must_use]
    pub fn summary(&self) -> DiffSummary {
        let mut summary = DiffSummary::default();
        for delta in &self.deltas {
            if delta.is_created() {
                summary.introduced += 1;
            } else if delta.is_removed() {
                summary.removed += 1;
            } else {
                summary.modified += 1;
            }
        }
        summary
    }
}

/// Compact deterministic counts for a semantic diff.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DiffSummary {
    /// Number of entities introduced on the right.
    pub introduced: usize,
    /// Number of entities removed from the right.
    pub removed: usize,
    /// Number of entities whose revision changed.
    pub modified: usize,
}

impl DiffSummary {
    /// Returns the total number of changed entities.
    #[must_use]
    pub const fn total_changed(self) -> usize {
        self.introduced + self.removed + self.modified
    }
}

/// Optional domain-specific interpretation of a universal diff.
pub trait DiffEnricher {
    /// Domain-owned interpretation returned for a universal diff.
    type Output;

    /// Enriches a diff without changing its authoritative deltas.
    ///
    /// # Parameters
    /// - `diff`: Universal diff with exact entity and revision references.
    ///
    /// # Returns
    /// Domain-specific interpretation of the diff.
    fn enrich(&self, diff: &SemanticDiff) -> Self::Output;
}

/// Compares two arbitrary snapshots through the shared storage contract.
///
/// Equal snapshot IDs use an immediate empty result. Other snapshots are
/// compared as state, with no ancestry or domain adapter requirement.
///
/// # Parameters
/// - `store`: Backend implementing snapshot enumeration.
/// - `left`: Snapshot on the left side.
/// - `right`: Snapshot on the right side.
///
/// # Returns
/// A deterministic semantic diff containing only changed entities.
///
/// # Errors
/// Returns a snapshot error when either exact snapshot cannot be read.
pub fn compare<S: SnapshotStore>(
    store: &S,
    left: SnapshotId,
    right: SnapshotId,
) -> Result<SemanticDiff, SnapshotError> {
    if left == right {
        return Ok(SemanticDiff::new(left, right, Vec::new()));
    }
    let before: BTreeMap<EntityId, _> = store.iter(left)?.into_iter().collect();
    let after: BTreeMap<EntityId, _> = store.iter(right)?.into_iter().collect();
    let mut deltas = Vec::new();
    let entities: BTreeSet<_> = before.keys().chain(after.keys()).copied().collect();
    for entity in entities {
        let old = before.get(&entity).copied();
        let new = after.get(&entity).copied();
        if old != new {
            if let Ok(delta) = EntityDelta::new(entity, old, new) {
                deltas.push(delta);
            }
        }
    }
    Ok(SemanticDiff::new(left, right, deltas))
}
