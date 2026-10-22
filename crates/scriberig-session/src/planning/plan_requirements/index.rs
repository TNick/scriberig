//! Maintains rebuildable plan rollups with ancestor-only updates.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::{EntityId, SnapshotId};

use super::super::{
    Plan, PlanAbandonReason, PlanError, PlanGatePhase, PlanGateStatus, PlanNodeParent,
    PlanNodeRollup, PlanNodeStatus,
};
use super::index_helpers::node_depth;
use super::rollups::{increment_status_count, rollup_status};

/// Holds snapshot-bound derived rollups outside canonical node revisions.
///
/// # Public members
/// None. Use the accessors and mutation methods to preserve snapshot validity.
///
/// # Private members
/// - `source_snapshot`: Canonical snapshot represented by the index.
/// - `evaluator_version`: Rollup evaluator version used to build the index.
/// - `rollups`: Effective status and direct-child counts by node.
/// - `parents`: Structural node-parent lookup.
/// - `children`: Structural adjacency lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanNodeRollupIndex {
    source_snapshot: SnapshotId,
    evaluator_version: u16,
    rollups: BTreeMap<EntityId, PlanNodeRollup>,
    parents: BTreeMap<EntityId, Option<EntityId>>,
    children: BTreeMap<EntityId, BTreeSet<EntityId>>,
}

impl PlanNodeRollupIndex {
    /// Restores a cached rollup graph for an exact plan snapshot.
    ///
    /// # Parameters
    /// - `plan`: Plan projection represented by the cache.
    /// - `source_snapshot`: Exact snapshot stored with the cache rows.
    /// - `rollups`: Cached values keyed by stable node identity.
    ///
    /// # Returns
    /// A reusable index with reconstructed structural topology.
    ///
    /// # Errors
    /// Returns an error when node membership or parent references are invalid.
    pub fn from_cached_rollups(
        plan: &Plan,
        source_snapshot: SnapshotId,
        rollups: BTreeMap<EntityId, PlanNodeRollup>,
    ) -> Result<Self, PlanError> {
        let node_ids = plan
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<BTreeSet<_>>();
        if node_ids.len() != plan.nodes.len()
            || node_ids != rollups.keys().copied().collect::<BTreeSet<_>>()
            || rollups.iter().any(|(id, value)| value.node != *id)
        {
            return Err(PlanError::StaleRollupIndex);
        }
        let mut parents = BTreeMap::new();
        let mut children = BTreeMap::<EntityId, BTreeSet<EntityId>>::new();
        for node in &plan.nodes {
            let parent = match node.parent {
                Some(PlanNodeParent::Node(parent)) if node_ids.contains(&parent) => Some(parent),
                Some(PlanNodeParent::Node(_)) => return Err(PlanError::MissingNode),
                Some(PlanNodeParent::Branch(_)) | None => None,
            };
            parents.insert(node.id, parent);
            children.entry(node.id).or_default();
            if let Some(parent) = parent {
                children.entry(parent).or_default().insert(node.id);
            }
        }
        Ok(Self {
            source_snapshot,
            evaluator_version: 2,
            rollups,
            parents,
            children,
        })
    }

    /// Builds a complete derived index from one exact plan snapshot.
    ///
    /// # Parameters
    /// - `plan`: Canonical plan projection resolved at `source_snapshot`.
    /// - `source_snapshot`: Exact universal snapshot represented by this index.
    ///
    /// A rebuildable index carrying reference rollup values.
    ///
    /// Returns a plan error when the structural graph is invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn rebuild(plan: &Plan, source_snapshot: SnapshotId) -> Result<Self, PlanError> {
        let rollups = plan.node_rollups()?;
        let mut parents = BTreeMap::new();
        let mut children = BTreeMap::<EntityId, BTreeSet<EntityId>>::new();
        for node in &plan.nodes {
            let parent = match node.parent {
                Some(PlanNodeParent::Node(parent)) => Some(parent),
                Some(PlanNodeParent::Branch(_)) | None => None,
            };
            parents.insert(node.id, parent);
            if let Some(parent) = parent {
                children.entry(parent).or_default().insert(node.id);
            }
        }
        Ok(Self {
            source_snapshot,
            evaluator_version: 2,
            rollups,
            parents,
            children,
        })
    }

    /// Returns the exact canonical snapshot represented by the index.
    #[must_use]
    pub const fn source_snapshot(&self) -> SnapshotId {
        self.source_snapshot
    }

    /// Returns the evaluator version required to interpret cached values.
    #[must_use]
    pub const fn evaluator_version(&self) -> u16 {
        self.evaluator_version
    }

    /// Returns the current derived rollup for one node.
    ///
    /// # Parameters
    /// - `self`: Snapshot-bound rollup index.
    /// - `node`: Stable plan-node identity.
    ///
    /// The derived node state, or `None` when the node is absent.
    ///
    /// # Returns
    /// The value described by the return type.
    #[must_use]
    pub fn get(&self, node: EntityId) -> Option<&PlanNodeRollup> {
        self.rollups.get(&node)
    }

    /// Returns every cached node rollup in stable identity order.
    ///
    /// # Returns
    /// The value described by the return type.
    #[must_use]
    pub fn all(&self) -> &BTreeMap<EntityId, PlanNodeRollup> {
        &self.rollups
    }

    /// Applies changed node state and propagates only changed ancestor paths.
    ///
    /// Structural changes are detected for each supplied node. The list must
    /// include every new node and every node whose direct state, gate owner,
    /// evidence owner, or parent relation changed. Node deletion is not
    /// supported by the current plan mutation model.
    ///
    /// # Parameters
    /// - `self`: Previously valid derived index.
    /// - `plan`: New canonical plan projection.
    /// - `expected_base`: Snapshot from which this update was derived.
    /// - `new_snapshot`: Exact snapshot represented after the update.
    /// - `changed_nodes`: Nodes whose canonical state or owned requirements changed.
    ///
    /// Unit after affected rollups and snapshot identity are updated.
    ///
    /// Returns a plan error for stale inputs, missing nodes, or structural cycles.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn apply_change(
        &mut self,
        plan: &Plan,
        expected_base: SnapshotId,
        new_snapshot: SnapshotId,
        changed_nodes: &[EntityId],
    ) -> Result<Vec<EntityId>, PlanError> {
        if self.source_snapshot != expected_base || self.evaluator_version != 2 {
            return Err(PlanError::StaleRollupIndex);
        }
        let mut proposed = self.clone();
        let mut affected_parents = BTreeSet::new();
        let mut touched = BTreeSet::new();
        let changed = changed_nodes.iter().copied().collect::<BTreeSet<_>>();
        if changed.len() != changed_nodes.len() {
            return Err(PlanError::InvalidTransition);
        }
        for node_id in &changed {
            let node = plan
                .nodes
                .iter()
                .find(|candidate| candidate.id == *node_id)
                .ok_or(PlanError::StaleRollupIndex)?;
            let new_parent = match node.parent {
                Some(PlanNodeParent::Node(parent)) => Some(parent),
                Some(PlanNodeParent::Branch(_)) | None => None,
            };
            let old_parent = proposed.parents.get(node_id).copied();
            if old_parent.flatten() != new_parent {
                touched.insert(*node_id);
                if let Some(parent) = old_parent.flatten() {
                    proposed.children.entry(parent).or_default().remove(node_id);
                    affected_parents.insert(parent);
                }
                if let Some(parent) = new_parent {
                    if !proposed.parents.contains_key(&parent)
                        && !changed.contains(&parent)
                        && !plan.nodes.iter().any(|candidate| candidate.id == parent)
                    {
                        return Err(PlanError::MissingNode);
                    }
                    proposed
                        .children
                        .entry(parent)
                        .or_default()
                        .insert(*node_id);
                    affected_parents.insert(parent);
                }
            }
            proposed.parents.insert(*node_id, new_parent);
            proposed.children.entry(*node_id).or_default();
        }
        if plan
            .nodes
            .iter()
            .any(|node| !proposed.parents.contains_key(&node.id) && !changed.contains(&node.id))
        {
            return Err(PlanError::StaleRollupIndex);
        }
        let mut changed_order = changed.into_iter().collect::<Vec<_>>();
        changed_order.sort_by_key(|node| std::cmp::Reverse(node_depth(plan, *node)));
        for node in changed_order {
            proposed.propagate(plan, node, false, &mut touched)?;
        }
        let mut parent_order = affected_parents.into_iter().collect::<Vec<_>>();
        parent_order.sort_by_key(|node| std::cmp::Reverse(node_depth(plan, *node)));
        for parent in parent_order {
            proposed.propagate(plan, parent, true, &mut touched)?;
        }
        proposed.source_snapshot = new_snapshot;
        *self = proposed;
        Ok(touched.into_iter().collect())
    }

    /// Recomputes one node and propagates changed values toward the root.
    ///
    /// # Parameters
    /// - `self`: Index being updated.
    /// - `plan`: New canonical plan projection.
    /// - `start`: First node to evaluate.
    /// - `force_parent`: Whether to continue after an unchanged first value.
    ///
    /// Unit after the path is updated through its first unchanged ancestor.
    ///
    /// Returns an error for missing cached descendants or a structural cycle.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    /// Propagates one cached change toward the structural root.
    ///
    /// # Parameters
    /// - `self`: Index being updated.
    /// - `plan`: Canonical plan projection at the target snapshot.
    /// - `start`: First node to evaluate.
    /// - `force_parent`: Whether to continue after an unchanged first value.
    /// - `touched`: Nodes whose derived values changed.
    ///
    /// # Returns
    /// Unit after updating the affected ancestor path.
    ///
    /// # Errors
    /// Returns an error for missing descendants or a structural cycle.
    fn propagate(
        &mut self,
        plan: &Plan,
        start: EntityId,
        force_parent: bool,
        touched: &mut BTreeSet<EntityId>,
    ) -> Result<(), PlanError> {
        let mut current = start;
        let mut force = force_parent;
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(current) {
                return Err(PlanError::StructuralCycle);
            }
            let next = self.compute(plan, current)?;
            let changed = self.rollups.insert(current, next) != Some(next);
            if changed {
                touched.insert(current);
            }
            if !changed && !force {
                break;
            }
            force = false;
            let Some(parent) = self.parents.get(&current).copied().flatten() else {
                break;
            };
            current = parent;
        }
        Ok(())
    }

    /// Computes one node from cached child rollups and current canonical gates.
    ///
    /// # Parameters
    /// - `self`: Index containing current direct-child values.
    /// - `plan`: Canonical plan projection at the target snapshot.
    /// - `node_id`: Node to recompute.
    ///
    /// The deterministic node rollup.
    ///
    /// Returns an error when the node or any direct child value is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compute(&self, plan: &Plan, node_id: EntityId) -> Result<PlanNodeRollup, PlanError> {
        let node = plan
            .nodes
            .iter()
            .find(|candidate| candidate.id == node_id)
            .ok_or(PlanError::MissingNode)?;
        let children = self.children.get(&node_id).into_iter().flatten();
        let mut counts = super::super::PlanNodeStatusCounts::default();
        let mut contains_ignored = node.abandon_reason == Some(PlanAbandonReason::Ignored);
        let mut contains_abandoned = node.status == PlanNodeStatus::Abandoned;
        for child in children {
            let rollup = self.rollups.get(child).ok_or(PlanError::StaleRollupIndex)?;
            increment_status_count(&mut counts, rollup.direct_status);
            contains_ignored |= rollup.contains_ignored;
            contains_abandoned |= rollup.contains_abandoned;
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
            if let super::super::PlanGateRequirement::RequiredChildCompleted { child } =
                &gate.requirement
                && let Some(rollup) = self.rollups.get(child)
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
}
