//! Assembles the shared `PlanView`/`PlanNodeView` semantics the Plans view renders.

use std::collections::BTreeMap;

use scriberig_core::EntityId;
use scriberig_session::{BranchResult, Plan, PlanNodeView, PlanView};

/// Shared plan and node views assembled for one projected plan copy.
///
/// The Plans view renders these values instead of deriving plan semantics
/// itself. The same `scriberig-session` view assembly backs the shared
/// `ApplicationService::read_plan_views` operation and the server plan view, so
/// the terminal cannot grow a second status, blocker, gate, evidence, or retry
/// rule.
///
/// # Public members
/// - `plan`: Shared plan view; `None` when shared assembly rejected this copy.
/// - `nodes`: Shared node views keyed by durable node identity.
///
/// # Private members
/// None.
pub(super) struct SharedPlanViews {
    /// Shared plan view for this plan copy.
    pub(super) plan: Option<PlanView>,
    /// Shared node views for this plan copy.
    pub(super) nodes: BTreeMap<EntityId, PlanNodeView>,
}

impl SharedPlanViews {
    /// Assembles the shared views for one durable plan copy.
    ///
    /// - `plan`: Canonical durable plan from the session snapshot.
    /// - `results`: Exact branch results visible in the same snapshot.
    ///
    /// A plan the shared assembly rejects (a structural cycle or a dangling
    /// structural parent) yields no shared views, so the projection keeps stable
    /// identity rows without inventing local status, blocker, or gate values.
    /// # Returns
    /// The shared views available for this plan copy.
    pub(super) fn assemble(plan: &Plan, results: &[BranchResult]) -> Self {
        let snapshot = plan.source_snapshot;
        let nodes = plan
            .node_rollups()
            .ok()
            .map(|rollups| {
                plan.nodes
                    .iter()
                    .filter_map(|node| {
                        plan.node_view(node.id, snapshot, Some(results), &rollups)
                            .ok()
                            .map(|view| (node.id, view))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            plan: plan.plan_view(snapshot).ok(),
            nodes,
        }
    }

    /// Shared view for one durable node identity.
    ///
    /// - `node`: Durable `PlanNode` identity.
    ///
    /// # Returns
    /// The shared node view when this plan copy has one.
    pub(super) fn node(&self, node: EntityId) -> Option<&PlanNodeView> {
        self.nodes.get(&node)
    }
}
