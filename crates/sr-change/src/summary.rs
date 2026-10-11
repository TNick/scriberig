//! Defines compact structural summaries for semantic change objects.

use crate::{Branch, DiffSummary, EntityDelta, MergeResult, Migration, SourceKind};
use scriberig_core::SnapshotId;

/// Compact facts for one exact migration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationSummary {
    /// Exact base snapshot.
    pub base: SnapshotId,
    /// Exact result snapshot.
    pub result: SnapshotId,
    /// Source category.
    pub source: SourceKind,
    /// Number of introduced entities.
    pub introduced: usize,
    /// Number of removed entities.
    pub removed: usize,
    /// Number of modified entities.
    pub modified: usize,
    /// Number of semantic operations.
    pub operation_count: usize,
    /// Number of validation evidence references.
    pub validation_count: usize,
}

/// Compact facts for one branch or workspace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchSummary {
    /// Stable branch identity.
    pub id: String,
    /// Branch base snapshot.
    pub base: SnapshotId,
    /// Current branch snapshot.
    pub current: SnapshotId,
    /// Branch status supplied by the caller.
    pub status: Option<String>,
    /// Authored title.
    pub title: Option<String>,
    /// Authored purpose.
    pub purpose: Option<String>,
    /// Authored description.
    pub description: Option<String>,
    /// Authored outcome.
    pub outcome: Option<String>,
    /// Resulting migration reference, when known.
    pub resulting_migration: Option<String>,
}

/// Compact facts for a three-way merge result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeSummary {
    /// Explicit merge base.
    pub base: SnapshotId,
    /// Left snapshot.
    pub left: SnapshotId,
    /// Right snapshot.
    pub right: SnapshotId,
    /// Number of resolved entity changes.
    pub resolved_count: usize,
    /// Number of unresolved conflicts.
    pub conflict_count: usize,
    /// Conflict counts grouped by stable kind.
    pub conflicts_by_kind: Vec<(String, usize)>,
}

impl Migration {
    /// Produces a compact summary without rendering domain values.
    ///
    /// # Returns
    /// Structural migration counts and exact snapshot handles.
    #[must_use]
    pub fn summary(&self) -> MigrationSummary {
        let counts = count_deltas(&self.deltas);
        MigrationSummary {
            base: self.base,
            result: self.result,
            source: self.provenance.source.clone(),
            introduced: counts.introduced,
            removed: counts.removed,
            modified: counts.modified,
            operation_count: self.operations.len(),
            validation_count: self.provenance.validation.len(),
        }
    }
}

impl Branch {
    /// Produces a compact branch summary preserving authored metadata.
    ///
    /// # Returns
    /// Structural branch information with optional descriptions unchanged.
    #[must_use]
    pub fn summary(&self) -> BranchSummary {
        BranchSummary {
            id: self.id.clone(),
            base: self.base,
            current: self.current,
            status: self.status.clone(),
            title: self.title.clone(),
            purpose: self.purpose.clone(),
            description: self.description.clone(),
            outcome: self.outcome.clone(),
            resulting_migration: self.resulting_migration.clone(),
        }
    }
}

impl MergeResult {
    /// Produces deterministic merge counts grouped by conflict kind.
    ///
    /// # Returns
    /// Structural merge facts without conflict prose generation.
    #[must_use]
    pub fn summary(&self) -> MergeSummary {
        let mut counts = std::collections::BTreeMap::new();
        for conflict in &self.conflicts {
            *counts.entry(format!("{:?}", conflict.kind)).or_insert(0) += 1;
        }
        MergeSummary {
            base: self.base,
            left: self.left,
            right: self.right,
            resolved_count: self.resolved_deltas.len(),
            conflict_count: self.conflicts.len(),
            conflicts_by_kind: counts.into_iter().collect(),
        }
    }
}

/// Counts entity transitions deterministically.
///
/// # Parameters
/// - `deltas`: Entity transitions to count.
///
/// # Returns
/// Compact introduced, removed, and modified counts.
fn count_deltas(deltas: &[EntityDelta]) -> DiffSummary {
    let mut summary = DiffSummary::default();
    for delta in deltas {
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
