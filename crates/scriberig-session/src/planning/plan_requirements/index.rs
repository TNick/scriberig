//! Maintains rebuildable plan rollups with ancestor-only updates.

use std::collections::{BTreeMap, BTreeSet};

use scriberig_core::{EntityId, SnapshotId};

use super::super::{Plan, PlanError, PlanNodeParent, PlanNodeRollup};

#[path = "index/update.rs"]
mod update;

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
    /// Restores an index from complete persisted rollups at one exact snapshot.
    ///
    /// # Parameters
    /// - `plan`: Canonical plan projection at `source_snapshot`.
    /// - `source_snapshot`: Exact snapshot represented by the cached values.
    /// - `rollups`: Validated cached rollups keyed by node identity.
    ///
    /// A sparse-update index with reconstructed parent and child maps.
    ///
    /// # Returns
    /// The index when every node has one matching cached rollup.
    ///
    /// # Errors
    /// Returns an error when node identities, direct statuses, or structure do
    /// not match the cached values.
    pub fn from_cached_rollups(
        plan: &Plan,
        source_snapshot: SnapshotId,
        rollups: BTreeMap<EntityId, PlanNodeRollup>,
    ) -> Result<Self, PlanError> {
        if rollups.len() != plan.nodes.len() {
            return Err(PlanError::StaleRollupIndex);
        }
        let mut parents = BTreeMap::new();
        let mut children = BTreeMap::<EntityId, BTreeSet<EntityId>>::new();
        for node in &plan.nodes {
            let parent = match node.parent {
                Some(PlanNodeParent::Node(parent)) => Some(parent),
                Some(PlanNodeParent::Branch(_)) | None => None,
            };
            let Some(rollup) = rollups.get(&node.id) else {
                return Err(PlanError::StaleRollupIndex);
            };
            if rollup.node != node.id || rollup.direct_status != node.status {
                return Err(PlanError::StaleRollupIndex);
            }
            if parents.insert(node.id, parent).is_some() {
                return Err(PlanError::DuplicateNode);
            }
            if let Some(parent) = parent {
                children.entry(parent).or_default().insert(node.id);
            }
        }
        if parents
            .values()
            .flatten()
            .any(|parent| !parents.contains_key(parent))
        {
            return Err(PlanError::MissingNode);
        }
        Ok(Self {
            source_snapshot,
            evaluator_version: 1,
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
            evaluator_version: 1,
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
    /// # Returns
    /// The cached rollup for the exact node identity, when present.
    #[must_use]
    pub fn get(&self, node: EntityId) -> Option<&PlanNodeRollup> {
        self.rollups.get(&node)
    }

    /// Returns every cached node rollup in stable identity order.
    /// # Returns
    /// All cached node rollups keyed by node identity.
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
    /// # Returns
    /// Node identities whose effective rollup values changed.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn apply_change(
        &mut self,
        plan: &Plan,
        expected_base: SnapshotId,
        new_snapshot: SnapshotId,
        changed_nodes: &[EntityId],
    ) -> Result<Vec<EntityId>, PlanError> {
        update::apply_change(self, plan, expected_base, new_snapshot, changed_nodes)
    }
}
/// Resolves the plan-node parent from a structural relation.
///
/// # Parameters
/// - `parent`: Branch or `PlanNode` structural relation.
///
/// The plan-node identity, or `None` for a plan root.
/// # Returns
/// The structural parent node, when the relation is a node.
fn node_parent(parent: Option<PlanNodeParent>) -> Option<EntityId> {
    match parent {
        Some(PlanNodeParent::Node(parent)) => Some(parent),
        Some(PlanNodeParent::Branch(_)) | None => None,
    }
}
