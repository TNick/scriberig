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
/// # Parameters
/// - `items`: Already deterministically ordered results.
/// - `offset`: Number of results to skip.
/// - `limit`: Maximum number of results to return.
///
/// # Returns
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
/// # Parameters
/// - `store`: Backend-neutral snapshot store.
/// - `left`: Earlier or left snapshot.
/// - `right`: Later or right snapshot.
///
/// # Returns
/// Stable entity and revision handles in a semantic diff.
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
/// # Parameters
/// - `history`: Explicit migration topology.
/// - `revision`: Revision to locate.
///
/// # Returns
/// Matching migration references in stable ID order.
#[must_use]
pub fn introduced_by(history: &HistoryGraph, revision: RevisionId) -> Vec<&MigrationReference> {
    history.introducing_revision(revision)
}

/// Returns all entity transitions in deterministic migration order.
///
/// # Parameters
/// - `history`: Explicit migration topology.
/// - `entity`: Entity to inspect.
///
/// # Returns
/// Exact before/after revisions and visible snapshots.
#[must_use]
pub fn entity_history(history: &HistoryGraph, entity: EntityId) -> Vec<LineageRecord> {
    history.lineage(entity)
}

/// Finds migrations touching one entity.
///
/// # Parameters
/// - `history`: Explicit migration topology.
/// - `entity`: Entity to inspect.
///
/// # Returns
/// Migration references in stable ID order.
#[must_use]
pub fn migrations_touching(history: &HistoryGraph, entity: EntityId) -> Vec<&MigrationReference> {
    history.touching_entity(entity)
}

/// Finds snapshots descending from an explicit ancestor.
///
/// # Parameters
/// - `history`: Explicit migration topology.
/// - `snapshot`: Ancestor snapshot.
///
/// # Returns
/// Descendant snapshots sorted by canonical identity.
#[must_use]
pub fn branches_descending_from(history: &HistoryGraph, snapshot: SnapshotId) -> Vec<SnapshotId> {
    history.descendants(snapshot)
}

/// Returns the most specific common ancestor in topology order.
///
/// # Parameters
/// - `history`: Explicit migration topology.
/// - `left`: First snapshot.
/// - `right`: Second snapshot.
///
/// # Returns
/// The highest canonical common ancestor, when one exists.
#[must_use]
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
/// # Parameters
/// - `base`: Explicit common base.
/// - `left`: Left snapshot.
/// - `right`: Right snapshot.
/// - `strategy`: Optional domain merge strategy.
///
/// # Returns
/// A structured merge result with resolved changes and conflicts.
///
/// # Errors
/// Returns an identity mismatch when supplied snapshot IDs are inconsistent.
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
/// # Parameters
/// - `history`: Explicit migration topology.
/// - `workspace`: Exact workspace identifier.
///
/// # Returns
/// Matching references in deterministic ID order.
#[must_use]
pub fn workspace_migrations<'a>(
    history: &'a HistoryGraph,
    workspace: &str,
) -> Vec<&'a MigrationReference> {
    history.in_workspace(workspace)
}
