//! Stages sparse SQLite rollup changes before semantic root promotion.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Transaction;
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{Plan, PlanEvidenceOwner, PlanGate, PlanNodeRollupIndex};

use crate::SqliteSessionStore;

/// Holds a validated successor rollup index and its rows to persist.
///
/// # Public members
/// None.
///
/// # Private members
/// - `index`: Complete successor values with sparse in-memory updates applied.
/// - `changed_rows`: Rollup rows whose derived value changed.
/// - `replace_all`: Whether the cache was absent or plan membership changed.
pub(super) struct StagedRollupIndex {
    index: PlanNodeRollupIndex,
    changed_rows: Vec<EntityId>,
    replace_all: bool,
}

/// Stages a cache update from a validated exact-snapshot predecessor.
///
/// A missing, incomplete, or obsolete cache selects a full rebuild. The
/// reference evaluator remains the fallback and recovery source.
///
/// # Parameters
/// - `store`: SQLite repository owning the snapshot-bound rollup cache.
/// - `session`: Owning durable session identity.
/// - `previous`: Canonical plan before the proposed mutation.
/// - `proposed`: Canonical plan being promoted.
/// - `old_snapshot`: Exact base snapshot.
/// - `new_snapshot`: Exact successor snapshot.
///
/// # Returns
/// A complete staged cache update suitable for the root-promotion transaction.
///
/// # Errors
/// Returns a diagnostic if canonical reference rollup computation fails.
pub(super) fn stage_rollup_update(
    store: &SqliteSessionStore,
    session: EntityId,
    previous: &Plan,
    proposed: &Plan,
    old_snapshot: SnapshotId,
    new_snapshot: SnapshotId,
) -> Result<StagedRollupIndex, String> {
    if node_ids(previous) != node_ids(proposed) {
        return full_rebuild(proposed, new_snapshot);
    }
    let Some(cached) = store
        .load_plan_rollups_at_snapshot(session, proposed.id, old_snapshot)
        .ok()
        .flatten()
    else {
        return full_rebuild(proposed, new_snapshot);
    };
    let Ok(mut index) = PlanNodeRollupIndex::from_cached_rollups(previous, old_snapshot, cached)
    else {
        return full_rebuild(proposed, new_snapshot);
    };
    let changed = changed_nodes(previous, proposed);
    let Ok(changed_rows) = index.apply_change(proposed, old_snapshot, new_snapshot, &changed)
    else {
        return full_rebuild(proposed, new_snapshot);
    };
    Ok(StagedRollupIndex {
        index,
        changed_rows,
        replace_all: false,
    })
}

/// Persists staged values and snapshot metadata in the root-promotion transaction.
///
/// # Parameters
/// - `transaction`: Transaction promoting the semantic root.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `staged`: Validated complete successor cache.
///
/// # Returns
/// Unit after changed values and exact-snapshot metadata are written.
///
/// # Errors
/// Returns a SQLite diagnostic if the normalized cache cannot be updated.
pub(super) fn persist_in_transaction(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    staged: &StagedRollupIndex,
) -> Result<(), String> {
    let snapshot = staged.index.source_snapshot();
    if staged.replace_all {
        return super::rollup_index::replace_in_transaction(
            transaction,
            session,
            plan,
            snapshot,
            staged.index.all(),
        );
    }
    for node in &staged.changed_rows {
        let rollup = staged
            .index
            .get(*node)
            .ok_or_else(|| "staged rollup row is missing".to_owned())?;
        super::rollup_index::write_rollup_row(transaction, session, plan, snapshot, rollup)?;
    }
    super::rollup_index::write_rollup_metadata(
        transaction,
        session,
        plan,
        snapshot,
        staged.index.all().len(),
    )
}

/// Builds a complete replacement from the backend-neutral reference evaluator.
///
/// # Parameters
/// - `plan`: Canonical plan projection at the target snapshot.
/// - `snapshot`: Exact source snapshot represented by the rebuilt cache.
///
/// # Returns
/// A full-replacement staged index.
///
/// # Errors
/// Returns a diagnostic when the canonical plan cannot be rolled up.
fn full_rebuild(plan: &Plan, snapshot: SnapshotId) -> Result<StagedRollupIndex, String> {
    let index = PlanNodeRollupIndex::rebuild(plan, snapshot)
        .map_err(|error| format!("plan rollup index rebuild failed: {error:?}"))?;
    let changed_rows = index.all().keys().copied().collect();
    Ok(StagedRollupIndex {
        index,
        changed_rows,
        replace_all: true,
    })
}

/// Returns stable node identities in one canonical plan.
///
/// # Parameters
/// - `plan`: Plan whose nodes are enumerated.
///
/// # Returns
/// The plan's node identities in sorted order.
fn node_ids(plan: &Plan) -> BTreeSet<EntityId> {
    plan.nodes.iter().map(|node| node.id).collect()
}

/// Finds node and gate owners whose inputs can change derived rollups.
///
/// # Parameters
/// - `previous`: Canonical plan before the mutation.
/// - `proposed`: Canonical plan after the mutation.
///
/// # Returns
/// Stable owner node identities requiring incremental evaluation.
fn changed_nodes(previous: &Plan, proposed: &Plan) -> Vec<EntityId> {
    let previous_nodes = previous
        .nodes
        .iter()
        .map(|node| (node.id, node))
        .collect::<BTreeMap<_, _>>();
    let mut changed = proposed
        .nodes
        .iter()
        .filter(|node| {
            previous_nodes
                .get(&node.id)
                .is_some_and(|old| *old != *node)
        })
        .map(|node| node.id)
        .collect::<BTreeSet<_>>();
    if previous.gates != proposed.gates || previous.evidence != proposed.evidence {
        changed.extend(previous.gates.iter().map(|gate| gate.owner));
        changed.extend(proposed.gates.iter().map(|gate| gate.owner));
        for evidence in previous.evidence.iter().chain(&proposed.evidence) {
            match evidence.owner {
                PlanEvidenceOwner::Node(node) => {
                    changed.insert(node);
                }
                PlanEvidenceOwner::Gate(gate) => {
                    add_gate_owner(&mut changed, &previous.gates, gate);
                    add_gate_owner(&mut changed, &proposed.gates, gate);
                }
            }
        }
    }
    changed.into_iter().collect()
}

/// Adds the owning node of a gate when the gate exists in the supplied plan.
///
/// # Parameters
/// - `nodes`: Staged set of nodes to recompute.
/// - `gates`: Gate definitions that may contain the association.
/// - `gate_id`: Stable gate identity referenced by evidence.
fn add_gate_owner(nodes: &mut BTreeSet<EntityId>, gates: &[PlanGate], gate_id: EntityId) {
    if let Some(gate) = gates.iter().find(|gate| gate.id == gate_id) {
        nodes.insert(gate.owner);
    }
}
