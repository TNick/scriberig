//! Applies sparse transactional changes to cached plan rollups.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::{EntityId, SnapshotId};

use super::super::super::{
    Plan, PlanAbandonReason, PlanError, PlanGatePhase, PlanGateStatus, PlanNodeRollup,
    PlanNodeStatus, PlanNodeStatusCounts,
};
use super::super::rollups::{increment_status_count, rollup_status};
use super::PlanNodeRollupIndex;

/// Stages affected nodes and ancestor values, then commits only sparse changes.
///
/// # Parameters
/// - `index`: Previously valid snapshot-bound index.
/// - `plan`: New canonical plan projection.
/// - `expected_base`: Snapshot from which the change was derived.
/// - `new_snapshot`: Exact snapshot represented after the change.
/// - `changed_nodes`: Nodes whose canonical state or owned requirements changed.
///
/// Node identities whose derived rollup value changed.
///
/// # Returns
/// Changed derived rollups when the sparse update is valid and committed.
///
/// # Errors
/// Returns a plan error for stale input, missing nodes, or structural cycles.
pub(super) fn apply_change(
    index: &mut PlanNodeRollupIndex,
    plan: &Plan,
    expected_base: SnapshotId,
    new_snapshot: SnapshotId,
    changed_nodes: &[EntityId],
) -> Result<Vec<EntityId>, PlanError> {
    if index.source_snapshot != expected_base || index.evaluator_version != 2 {
        return Err(PlanError::StaleRollupIndex);
    }

    let changed = changed_nodes.iter().copied().collect::<BTreeSet<_>>();
    if changed.len() != changed_nodes.len() {
        return Err(PlanError::InvalidTransition);
    }
    let new_node_count = changed
        .iter()
        .filter(|node| !index.parents.contains_key(node))
        .count();
    if plan.nodes.len() != index.parents.len() + new_node_count {
        return Err(PlanError::StaleRollupIndex);
    }

    let mut parent_updates = BTreeMap::new();
    let mut children_updates = BTreeMap::new();
    let mut affected_parents = BTreeSet::new();
    for node_id in &changed {
        let node = plan
            .nodes
            .iter()
            .find(|node| node.id == *node_id)
            .ok_or(PlanError::StaleRollupIndex)?;
        let new_parent = super::node_parent(node.parent);
        let old_parent = index.parents.get(node_id).copied();
        if old_parent.flatten() != new_parent {
            if let Some(parent) = old_parent.flatten() {
                children_mut(index, &mut children_updates, parent).remove(node_id);
                affected_parents.insert(parent);
            }
            if let Some(parent) = new_parent {
                if !index.parents.contains_key(&parent) && !changed.contains(&parent) {
                    return Err(PlanError::MissingNode);
                }
                children_mut(index, &mut children_updates, parent).insert(*node_id);
                affected_parents.insert(parent);
            }
        }
        parent_updates.insert(*node_id, new_parent);
        children_updates
            .entry(*node_id)
            .or_insert_with(|| index.children.get(node_id).cloned().unwrap_or_default());
    }

    let mut depths = BTreeMap::new();
    for node in changed.iter().chain(affected_parents.iter()) {
        depths.insert(*node, node_depth(*node, &parent_updates, &index.parents)?);
    }
    let mut changed_order = changed.into_iter().collect::<Vec<_>>();
    changed_order.sort_by_key(|node| std::cmp::Reverse(depths[node]));
    let mut parent_order = affected_parents.into_iter().collect::<Vec<_>>();
    parent_order.sort_by_key(|node| std::cmp::Reverse(depths[node]));

    let mut rollup_updates = BTreeMap::new();
    for node in changed_order {
        propagate(
            index,
            plan,
            node,
            false,
            &parent_updates,
            &children_updates,
            &mut rollup_updates,
        )?;
    }
    for node in parent_order {
        propagate(
            index,
            plan,
            node,
            true,
            &parent_updates,
            &children_updates,
            &mut rollup_updates,
        )?;
    }

    let changed_rollups = rollup_updates
        .iter()
        .filter_map(|(node, rollup)| (index.rollups.get(node) != Some(rollup)).then_some(*node))
        .collect::<Vec<_>>();
    index.parents.extend(parent_updates);
    index.children.extend(children_updates);
    index.rollups.extend(rollup_updates);
    index.source_snapshot = new_snapshot;
    Ok(changed_rollups)
}

/// Returns a staged child set, cloning only the one affected adjacency row.
///
/// # Parameters
/// - `index`: Existing index supplying an unchanged row when present.
/// - `updates`: Sparse child-set overlay being assembled.
/// - `parent`: Parent whose adjacency row is requested.
///
/// The mutable staged child set for `parent`.
///
/// # Returns
/// The staged row, initialized from the index or as an empty set.
fn children_mut<'a>(
    index: &PlanNodeRollupIndex,
    updates: &'a mut BTreeMap<EntityId, BTreeSet<EntityId>>,
    parent: EntityId,
) -> &'a mut BTreeSet<EntityId> {
    updates
        .entry(parent)
        .or_insert_with(|| index.children.get(&parent).cloned().unwrap_or_default())
}

/// Recomputes one node and stages changed ancestor rollups.
///
/// # Parameters
/// - `index`: Existing values used for unchanged descendants.
/// - `plan`: Canonical plan projection at the target snapshot.
/// - `start`: First node to evaluate.
/// - `force_parent`: Whether to continue after an unchanged first value.
/// - `parent_updates`: Staged structural-parent overrides.
/// - `children_updates`: Staged adjacency rows.
/// - `rollup_updates`: Staged rollup values.
///
/// Unit after the changed path is staged through its first unchanged ancestor.
///
/// # Returns
/// Unit when the affected path is valid.
///
/// # Errors
/// Returns a plan error when a required node or child rollup is missing.
fn propagate(
    index: &PlanNodeRollupIndex,
    plan: &Plan,
    start: EntityId,
    force_parent: bool,
    parent_updates: &BTreeMap<EntityId, Option<EntityId>>,
    children_updates: &BTreeMap<EntityId, BTreeSet<EntityId>>,
    rollup_updates: &mut BTreeMap<EntityId, PlanNodeRollup>,
) -> Result<(), PlanError> {
    let mut current = start;
    let mut force = force_parent;
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(current) {
            return Err(PlanError::StructuralCycle);
        }
        let next = compute(index, plan, current, children_updates, rollup_updates)?;
        let previous = rollup_updates
            .get(&current)
            .or_else(|| index.rollups.get(&current));
        let changed = previous != Some(&next);
        rollup_updates.insert(current, next);
        if !changed && !force {
            break;
        }
        force = false;
        let Some(parent) = parent_updates
            .get(&current)
            .copied()
            .or_else(|| index.parents.get(&current).copied())
            .flatten()
        else {
            break;
        };
        current = parent;
    }
    Ok(())
}

/// Computes one node using staged child and rollup values.
///
/// # Parameters
/// - `index`: Existing rollups for unchanged descendants.
/// - `plan`: Canonical plan projection at the target snapshot.
/// - `node_id`: Node to evaluate.
/// - `children_updates`: Staged adjacency rows.
/// - `rollup_updates`: Staged values for changed descendants.
///
/// The deterministic rollup for `node_id`.
///
/// # Returns
/// The node rollup when all required child values exist.
///
/// # Errors
/// Returns `MissingNode` or `StaleRollupIndex` when required values are absent.
fn compute(
    index: &PlanNodeRollupIndex,
    plan: &Plan,
    node_id: EntityId,
    children_updates: &BTreeMap<EntityId, BTreeSet<EntityId>>,
    rollup_updates: &BTreeMap<EntityId, PlanNodeRollup>,
) -> Result<PlanNodeRollup, PlanError> {
    let node = plan
        .nodes
        .iter()
        .find(|candidate| candidate.id == node_id)
        .ok_or(PlanError::MissingNode)?;
    let children = children_updates
        .get(&node_id)
        .or_else(|| index.children.get(&node_id));
    let mut counts = PlanNodeStatusCounts::default();
    let mut contains_ignored = node.abandon_reason == Some(PlanAbandonReason::Ignored);
    let mut contains_abandoned = node.status == PlanNodeStatus::Abandoned;
    if let Some(children) = children {
        for child in children {
            let rollup = rollup_updates
                .get(child)
                .or_else(|| index.rollups.get(child))
                .ok_or(PlanError::StaleRollupIndex)?;
            increment_status_count(&mut counts, rollup.direct_status);
            contains_ignored |= rollup.contains_ignored;
            contains_abandoned |= rollup.contains_abandoned;
        }
    }
    let required_gates = plan
        .gates
        .iter()
        .filter(|gate| gate.owner == node_id && gate.phase == PlanGatePhase::Completion)
        .collect::<Vec<_>>();
    let unsatisfied = required_gates
        .iter()
        .filter(|gate| {
            plan.evaluate_gate(gate.id).is_none_or(|evaluation| {
                !matches!(
                    evaluation.status,
                    PlanGateStatus::Satisfied | PlanGateStatus::Waived
                )
            })
        })
        .count();
    let mut relevant_children = Vec::new();
    for gate in required_gates {
        if let super::super::super::PlanGateRequirement::RequiredChildCompleted { child } =
            &gate.requirement
            && let Some(rollup) = rollup_updates
                .get(child)
                .or_else(|| index.rollups.get(child))
        {
            relevant_children.push(rollup);
        }
    }
    Ok(PlanNodeRollup {
        node: node_id,
        direct_status: node.status,
        effective_status: rollup_status(
            node.status,
            &relevant_children,
            u32::try_from(unsatisfied).unwrap_or(u32::MAX),
        ),
        contains_ignored,
        contains_abandoned,
        child_counts: counts,
        unsatisfied_completion_gates: u32::try_from(unsatisfied).unwrap_or(u32::MAX),
        entry_gates: plan.gate_counts(node_id, PlanGatePhase::Entry),
        completion_gates: plan.gate_counts(node_id, PlanGatePhase::Completion),
        evidence_count: plan.evidence_count(node_id),
    })
}

/// Calculates structural depth through the staged parent map.
///
/// # Parameters
/// - `node`: Node whose depth is requested.
/// - `parent_updates`: Staged parent overrides.
/// - `parents`: Existing structural-parent lookup.
///
/// The number of `PlanNode` ancestors before the branch/root boundary.
///
/// # Returns
/// The node depth in the proposed structure.
///
/// # Errors
/// Returns `StructuralCycle` or `MissingNode` for invalid parent paths.
fn node_depth(
    node: EntityId,
    parent_updates: &BTreeMap<EntityId, Option<EntityId>>,
    parents: &BTreeMap<EntityId, Option<EntityId>>,
) -> Result<usize, PlanError> {
    let mut current = node;
    let mut depth = 0;
    let mut visited = BTreeSet::new();
    while visited.insert(current) {
        let parent = parent_updates
            .get(&current)
            .copied()
            .or_else(|| parents.get(&current).copied())
            .ok_or(PlanError::MissingNode)?;
        let Some(parent) = parent else {
            return Ok(depth);
        };
        current = parent;
        depth += 1;
    }
    Err(PlanError::StructuralCycle)
}
