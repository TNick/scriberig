//! Defines backend-independent structured questions over change history.

use crate::history::{HistoryGraph, LineageRecord, MigrationReference};
use crate::{DomainMergeStrategy, MergeInput, MergeResult, SemanticDiff, compare, merge_snapshots};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_store::{Snapshot, SnapshotError, SnapshotStore};

/// A bounded deterministic page of query results.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryPage<T> {
    /// Results in canonical query order.
    pub items: Vec<T>,
    /// Offset cursor for the next page, when more results exist.
    pub next_cursor: Option<usize>,
}

/// Returns one bounded page without changing result meaning.
///
/// - `items`: Already deterministically ordered results.
/// - `offset`: Number of results to skip.
/// - `limit`: Maximum number of results to return.
///
/// A page and an offset cursor when more values remain.
#[must_use]
pub fn page<T: Clone>(items: &[T], offset: usize, limit: usize) -> QueryPage<T> {
    let end = offset.saturating_add(limit).min(items.len());
    QueryPage {
        items: items
            .get(offset.min(items.len())..end)
            .unwrap_or_default()
            .to_vec(),
        next_cursor: (end < items.len()).then_some(end),
    }
}

/// Answers what changed between two exact snapshots.
///
/// - `store`: Backend-neutral snapshot store.
/// - `left`: Earlier or left snapshot.
/// - `right`: Later or right snapshot.
///
/// Stable entity and revision handles in a semantic diff.
///
/// # Returns
/// The exact semantic changes between the requested snapshots.
///
/// # Errors
/// Returns a snapshot read error.
pub fn changed_between<S: SnapshotStore>(
    store: &S,
    left: SnapshotId,
    right: SnapshotId,
) -> Result<SemanticDiff, SnapshotError> {
    compare(store, left, right)
}

/// Finds the migrations that introduced a revision.
///
/// - `history`: Explicit migration topology.
/// - `revision`: Revision to locate.
///
/// Matching migration references in stable ID order.
#[must_use]
/// # Returns
/// The operation result.
pub fn introduced_by(history: &HistoryGraph, revision: RevisionId) -> Vec<&MigrationReference> {
    history.introducing_revision(revision)
}

/// Returns all entity transitions in deterministic migration order.
///
/// - `history`: Explicit migration topology.
/// - `entity`: Entity to inspect.
///
/// Exact before/after revisions and visible snapshots.
#[must_use]
/// # Returns
/// The operation result.
pub fn entity_history(history: &HistoryGraph, entity: EntityId) -> Vec<LineageRecord> {
    history.lineage(entity)
}

/// Finds migrations touching one entity.
///
/// - `history`: Explicit migration topology.
/// - `entity`: Entity to inspect.
///
/// Migration references in stable ID order.
#[must_use]
/// # Returns
/// The operation result.
pub fn migrations_touching(history: &HistoryGraph, entity: EntityId) -> Vec<&MigrationReference> {
    history.touching_entity(entity)
}

/// Finds snapshots descending from an explicit ancestor.
///
/// - `history`: Explicit migration topology.
/// - `snapshot`: Ancestor snapshot.
///
/// Descendant snapshots sorted by canonical identity.
#[must_use]
/// # Returns
/// The operation result.
pub fn branches_descending_from(history: &HistoryGraph, snapshot: SnapshotId) -> Vec<SnapshotId> {
    history.descendants(snapshot)
}

/// Returns the most specific common ancestor in topology order.
///
/// - `history`: Explicit migration topology.
/// - `left`: First snapshot.
/// - `right`: Second snapshot.
///
/// The highest canonical common ancestor, when one exists.
#[must_use]
/// # Returns
/// The operation result.
pub fn common_ancestor(
    history: &HistoryGraph,
    left: SnapshotId,
    right: SnapshotId,
) -> Option<SnapshotId> {
    let common = history.common_ancestors(left, right);
    common
        .iter()
        .copied()
        .filter(|candidate| {
            !common
                .iter()
                .any(|other| *candidate != *other && history.is_ancestor(*candidate, *other))
        })
        .max()
}

/// Returns conflicts that prevent merging three exact snapshots.
///
/// - `base`: Explicit common base.
/// - `left`: Left snapshot.
/// - `right`: Right snapshot.
/// - `strategy`: Optional domain merge strategy.
///
/// A structured merge result with resolved changes and conflicts.
///
/// Returns an identity mismatch when supplied snapshot IDs are inconsistent.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn merge_conflicts(
    base: &Snapshot,
    left: &Snapshot,
    right: &Snapshot,
    strategy: Option<&dyn DomainMergeStrategy>,
) -> Result<MergeResult, crate::MergeError> {
    merge_snapshots(
        MergeInput {
            base: base.id(),
            left: left.id(),
            right: right.id(),
        },
        base,
        left,
        right,
        strategy,
    )
}

/// Filters migration references to one workspace provenance value.
///
/// - `history`: Explicit migration topology.
/// - `workspace`: Exact workspace identifier.
///
/// Matching references in deterministic ID order.
#[must_use]
pub fn workspace_migrations<'a>(
    history: &'a HistoryGraph,
    workspace: &str,
) -> Vec<&'a MigrationReference> {
    history.in_workspace(workspace)
}
