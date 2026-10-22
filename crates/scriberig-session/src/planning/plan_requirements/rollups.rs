//! Computes deterministic effective statuses from plan structure and gates.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::EntityId;

use super::super::{
    Plan, PlanAbandonReason, PlanError, PlanGatePhase, PlanGateStatus, PlanNodeParent,
    PlanNodeStatus,
};

/// Counts direct children by the one canonical lifecycle status enum.
///
/// # Public members
/// - `planned`: Number of direct Planned children.
/// - `ready`: Number of direct Ready children.
/// - `running`: Number of direct Running children.
/// - `completed`: Number of direct Completed children.
/// - `failed`: Number of direct Failed children.
/// - `abandoned`: Number of direct Abandoned children.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanNodeStatusCounts {
    /// Number of direct Planned children.
    pub planned: u32,
    /// Number of direct Ready children.
    pub ready: u32,
    /// Number of direct Running children.
    pub running: u32,
    /// Number of direct Completed children.
    pub completed: u32,
    /// Number of direct Failed children.
    pub failed: u32,
    /// Number of direct Abandoned children.
    pub abandoned: u32,
}

/// Holds rebuildable effective subtree state for one plan node.
///
/// # Public members
/// - `node`: Stable PlanNode identity.
/// - `direct_status`: Canonical state stored by the node.
/// - `effective_status`: Derived status using PlanNodeStatus values.
/// - `contains_ignored`: Whether this subtree contains ignored work.
/// - `contains_abandoned`: Whether this subtree contains abandoned work.
/// - `child_counts`: Direct-child counts by canonical status.
/// - `unsatisfied_completion_gates`: Pending/failed completion gate count.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PlanNodeRollup {
    /// Stable PlanNode identity.
    pub node: EntityId,
    /// Canonical state stored by the node.
    pub direct_status: PlanNodeStatus,
    /// Derived status using PlanNodeStatus values.
    pub effective_status: PlanNodeStatus,
    /// Whether this subtree contains ignored work.
    pub contains_ignored: bool,
    /// Whether this subtree contains abandoned work.
    pub contains_abandoned: bool,
    /// Direct-child counts by canonical status.
    pub child_counts: PlanNodeStatusCounts,
    /// Pending/failed completion gate count.
    pub unsatisfied_completion_gates: u32,
}

/// Computes reference subtree rollups for every node in one plan snapshot.
///
/// Abandoned ignored descendants set `contains_ignored` without forcing their
/// ancestors to Abandoned. Required child gates still block parent completion.
///
/// # Parameters
/// - `self`: Plan snapshot with exact structural and gate state.
///
/// A deterministic rollup for every PlanNode.
///
/// Returns `StructuralCycle` for invalid cyclic parent relations.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
impl Plan {
    pub fn node_rollups(&self) -> Result<BTreeMap<EntityId, PlanNodeRollup>, PlanError> {
        let mut children = BTreeMap::<EntityId, Vec<EntityId>>::new();
        for node in &self.nodes {
            if let Some(PlanNodeParent::Node(parent)) = node.parent {
                if !self.nodes.iter().any(|candidate| candidate.id == parent) {
                    return Err(PlanError::MissingNode);
                }
                children.entry(parent).or_default().push(node.id);
            }
        }
        for siblings in children.values_mut() {
            siblings.sort();
        }
        let mut rollups = BTreeMap::new();
        let mut visiting = BTreeSet::new();
        for node in &self.nodes {
            compute_rollup(self, node.id, &children, &mut visiting, &mut rollups)?;
        }
        Ok(rollups)
    }
}

/// Recursively computes one node and its descendants with cycle detection.
///
/// # Parameters
/// - `plan`: Exact plan snapshot used for gate evaluation.
/// - `node_id`: Node whose subtree is computed.
/// - `children`: Structural child adjacency.
/// - `visiting`: Current DFS path used to reject cycles.
/// - `rollups`: Memoized results for already-computed nodes.
///
/// The node's effective rollup.
///
/// Returns `StructuralCycle` for a parent cycle or `MissingNode` for a dangling
/// structural reference.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn compute_rollup(
    plan: &Plan,
    node_id: EntityId,
    children: &BTreeMap<EntityId, Vec<EntityId>>,
    visiting: &mut BTreeSet<EntityId>,
    rollups: &mut BTreeMap<EntityId, PlanNodeRollup>,
) -> Result<PlanNodeRollup, PlanError> {
    if let Some(existing) = rollups.get(&node_id) {
        return Ok(*existing);
    }
    if !visiting.insert(node_id) {
        return Err(PlanError::StructuralCycle);
    }
    let node = plan
        .nodes
        .iter()
        .find(|candidate| candidate.id == node_id)
        .ok_or(PlanError::MissingNode)?;
    let mut child_rollups = Vec::new();
    for child in children.get(&node_id).into_iter().flatten() {
        child_rollups.push(compute_rollup(
            plan, *child, children, visiting, rollups,
        )?);
    }
    visiting.remove(&node_id);
    let mut counts = PlanNodeStatusCounts::default();
    for child in &child_rollups {
        increment_status_count(&mut counts, child.direct_status);
    }
    let contains_ignored = node.abandon_reason == Some(PlanAbandonReason::Ignored)
        || child_rollups.iter().any(|child| child.contains_ignored);
    let contains_abandoned = node.status == PlanNodeStatus::Abandoned
        || child_rollups.iter().any(|child| child.contains_abandoned);
    let required_children = plan
        .gates
        .iter()
        .filter(|gate| {
            gate.owner == node_id
                && gate.phase == PlanGatePhase::Completion
                && matches!(
                    gate.requirement,
                    super::super::PlanGateRequirement::RequiredChildCompleted { .. }
                )
        })
        .collect::<Vec<_>>();
    let unsatisfied_completion_gates = required_children
        .iter()
        .filter(|gate| {
            plan.evaluate_gate(gate.id).is_none_or(|evaluation| {
                !matches!(
                    evaluation.status,
                    PlanGateStatus::Satisfied | PlanGateStatus::Waived
                )
            })
        })
        .count() as u32;
    let relevant_children = child_rollups
        .iter()
        .filter(|child| {
            required_children.iter().any(|gate| {
                matches!(
                    gate.requirement,
                    super::super::PlanGateRequirement::RequiredChildCompleted { child: required }
                        if required == child.node
                )
            })
        })
        .collect::<Vec<_>>();
    let effective_status = rollup_status(
        node.status,
        &relevant_children,
        unsatisfied_completion_gates,
    );
    let rollup = PlanNodeRollup {
        node: node_id,
        direct_status: node.status,
        effective_status,
        contains_ignored,
        contains_abandoned,
        child_counts: counts,
        unsatisfied_completion_gates,
    };
    rollups.insert(node_id, rollup);
    Ok(rollup)
}

/// Applies the documented precedence to direct and required-descendant state.
///
/// # Parameters
/// - `direct`: Node's canonical direct status.
/// - `children`: Required structural descendants' effective statuses.
/// - `unsatisfied_gates`: Pending or failed completion gates.
///
/// The deterministic effective status.
fn rollup_status(
    direct: PlanNodeStatus,
    children: &[&PlanNodeRollup],
    unsatisfied_gates: u32,
) -> PlanNodeStatus {
    if direct == PlanNodeStatus::Abandoned {
        return PlanNodeStatus::Abandoned;
    }
    let statuses = children
        .iter()
        .map(|child| child.effective_status)
        .chain(std::iter::once(direct))
        .collect::<Vec<_>>();
    for status in [
        PlanNodeStatus::Failed,
        PlanNodeStatus::Running,
        PlanNodeStatus::Ready,
        PlanNodeStatus::Planned,
    ] {
        if statuses.contains(&status) {
            return status;
        }
    }
    if unsatisfied_gates > 0 {
        PlanNodeStatus::Planned
    } else {
        PlanNodeStatus::Completed
    }
}

/// Increments the direct child count for one canonical status.
///
/// # Parameters
/// - `counts`: Direct child status accumulator.
/// - `status`: Canonical child status to count.
fn increment_status_count(counts: &mut PlanNodeStatusCounts, status: PlanNodeStatus) {
    match status {
        PlanNodeStatus::Planned => counts.planned += 1,
        PlanNodeStatus::Ready => counts.ready += 1,
        PlanNodeStatus::Running => counts.running += 1,
        PlanNodeStatus::Completed => counts.completed += 1,
        PlanNodeStatus::Failed => counts.failed += 1,
        PlanNodeStatus::Abandoned => counts.abandoned += 1,
    }
}
