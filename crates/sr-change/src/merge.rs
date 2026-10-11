//! Implements backend-independent three-way semantic merge.

use crate::{ConflictKind, EntityDelta, MergeConflict};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_store::{Snapshot, SnapshotChange, SnapshotError, SnapshotStore};
use std::collections::BTreeSet;

/// Exact snapshots supplied to a three-way merge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MergeInput {
    /// Explicit common base snapshot identity.
    pub base: SnapshotId,
    /// Left branch snapshot identity.
    pub left: SnapshotId,
    /// Right branch snapshot identity.
    pub right: SnapshotId,
}

/// Exact state supplied to an optional domain merge strategy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityMergeContext {
    /// Entity being merged.
    pub entity: EntityId,
    /// Base revision, if present.
    pub base: Option<RevisionId>,
    /// Left revision, if present.
    pub left: Option<RevisionId>,
    /// Right revision, if present.
    pub right: Option<RevisionId>,
}

/// Result returned by a domain strategy for one conflict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainMergeOutcome {
    /// Strategy proved one revision safe.
    Resolved(Option<RevisionId>),
    /// Strategy deliberately retained a structured conflict.
    Conflict(String),
    /// Strategy does not apply to this entity.
    NotApplicable,
}

/// Optional domain extension for safe same-entity merge.
pub trait DomainMergeStrategy {
    /// Resolves one exact conflicted entity without mutating snapshots.
    /// # Parameters
    /// - `context`: Exact base, left, and right revisions.
    /// # Returns
    /// A resolution, explicit conflict detail, or not-applicable result.
    fn resolve(&self, context: EntityMergeContext) -> DomainMergeOutcome;
}

/// Successful three-way merge result without automatic promotion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeResult {
    /// Explicit merge base.
    pub base: SnapshotId,
    /// Left input snapshot.
    pub left: SnapshotId,
    /// Right input snapshot.
    pub right: SnapshotId,
    /// Automatically or strategy-resolved entity deltas.
    pub resolved_deltas: Vec<EntityDelta>,
    /// Unresolved semantic conflicts.
    pub conflicts: Vec<MergeConflict>,
    /// Retained merged snapshot when materialization completed without conflicts.
    pub result: Option<SnapshotId>,
}

/// Reports invalid merge input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MergeError {
    /// Supplied snapshots do not match the identities in the request.
    InputIdentityMismatch,
}

/// Merges three immutable snapshots using universal entity rules.
/// # Parameters
/// - `input`: IDs describing the merge request.
/// - `base`: Explicit base snapshot.
/// - `left`: Left branch snapshot.
/// - `right`: Right branch snapshot.
/// - `strategy`: Optional domain strategy.
/// # Returns
/// Resolved deltas and minimal unresolved conflicts.
/// # Errors
/// Returns an identity error when inputs do not match the request IDs.
pub fn merge_snapshots(
    input: MergeInput,
    base: &Snapshot,
    left: &Snapshot,
    right: &Snapshot,
    strategy: Option<&dyn DomainMergeStrategy>,
) -> Result<MergeResult, MergeError> {
    if input.base != base.id() || input.left != left.id() || input.right != right.id() {
        return Err(MergeError::InputIdentityMismatch);
    }
    let base_map: std::collections::BTreeMap<_, _> = base.iter().collect();
    let left_map: std::collections::BTreeMap<_, _> = left.iter().collect();
    let right_map: std::collections::BTreeMap<_, _> = right.iter().collect();
    let entities: BTreeSet<_> = base_map
        .keys()
        .chain(left_map.keys())
        .chain(right_map.keys())
        .copied()
        .collect();
    let mut result = MergeResult {
        base: input.base,
        left: input.left,
        right: input.right,
        resolved_deltas: Vec::new(),
        conflicts: Vec::new(),
        result: None,
    };
    for entity in entities {
        let b = base_map.get(&entity).copied();
        let l = left_map.get(&entity).copied();
        let r = right_map.get(&entity).copied();
        let left_changed = l != b;
        let right_changed = r != b;
        if !left_changed && !right_changed {
            continue;
        }
        if left_changed && !right_changed {
            add_delta(&mut result.resolved_deltas, entity, b, l);
            continue;
        }
        if right_changed && !left_changed {
            add_delta(&mut result.resolved_deltas, entity, b, r);
            continue;
        }
        if l == r {
            add_delta(&mut result.resolved_deltas, entity, b, l);
            continue;
        }
        let kind = if b.is_none() {
            ConflictKind::CompetingCreate
        } else if l.is_none() || r.is_none() {
            ConflictKind::DeleteModify
        } else {
            ConflictKind::BothModified
        };
        let context = EntityMergeContext {
            entity,
            base: b,
            left: l,
            right: r,
        };
        if let Some(strategy) = strategy {
            match strategy.resolve(context) {
                DomainMergeOutcome::Resolved(revision) => {
                    add_delta(&mut result.resolved_deltas, entity, b, revision);
                    continue;
                }
                DomainMergeOutcome::Conflict(detail) => {
                    result.conflicts.push(MergeConflict {
                        entity,
                        base: b,
                        left: l,
                        right: r,
                        kind: ConflictKind::DomainConflict,
                        path: None,
                        detail: Some(detail),
                    });
                    continue;
                }
                DomainMergeOutcome::NotApplicable => {}
            }
        }
        result.conflicts.push(MergeConflict {
            entity,
            base: b,
            left: l,
            right: r,
            kind,
            path: None,
            detail: None,
        });
    }
    Ok(result)
}

/// Reports merge materialization failures from the storage boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MergeStoreError {
    /// One requested input snapshot could not be read or the result could not be stored.
    Snapshot(SnapshotError),
    /// The in-memory merge input was inconsistent.
    Merge(MergeError),
}

/// Performs a three-way merge and retains its conflict-free result snapshot.
///
/// # Parameters
/// - `store`: Backend-neutral snapshot store receiving the result.
/// - `input`: Exact base, left, and right snapshot identities.
/// - `strategy`: Optional domain strategy for ambiguous entities.
///
/// # Returns
/// Merge deltas and the retained result identity when no conflicts remain.
///
/// # Errors
/// Returns a merge-input or snapshot-store error.
pub fn merge_snapshots_into_store<S: SnapshotStore>(
    store: &S,
    input: MergeInput,
    strategy: Option<&dyn DomainMergeStrategy>,
) -> Result<MergeResult, MergeStoreError> {
    let base = snapshot_from_store(store, input.base)?;
    let left = snapshot_from_store(store, input.left)?;
    let right = snapshot_from_store(store, input.right)?;
    let mut result =
        merge_snapshots(input, &base, &left, &right, strategy).map_err(MergeStoreError::Merge)?;
    if result.conflicts.is_empty() {
        let changes = result
            .resolved_deltas
            .iter()
            .map(|delta| match delta.after {
                Some(revision) => SnapshotChange::Set(delta.entity, revision),
                None => SnapshotChange::Remove(delta.entity),
            })
            .collect::<Vec<_>>();
        result.result = Some(
            store
                .apply_changes(input.base, &changes)
                .map_err(MergeStoreError::Snapshot)?,
        );
    }
    Ok(result)
}

/// Reconstructs one immutable snapshot through the public store interface.
fn snapshot_from_store<S: SnapshotStore>(
    store: &S,
    id: SnapshotId,
) -> Result<Snapshot, MergeStoreError> {
    let entries = store.iter(id).map_err(MergeStoreError::Snapshot)?;
    Ok(Snapshot::from_entries(entries.into_iter().collect()))
}

/// Adds a delta when the transition is semantically non-empty.
///
/// # Parameters
/// - `deltas`: Output delta collection.
/// - `entity`: Changed entity.
/// - `before`: Base revision.
/// - `after`: Result revision.
fn add_delta(
    deltas: &mut Vec<EntityDelta>,
    entity: EntityId,
    before: Option<RevisionId>,
    after: Option<RevisionId>,
) {
    if let Ok(delta) = EntityDelta::new(entity, before, after) {
        deltas.push(delta);
    }
}
