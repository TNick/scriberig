//! Holds structural helpers for incremental plan rollups.

use std::collections::BTreeSet;

use scriberig_core::EntityId;

use super::super::{Plan, PlanNodeParent};

/// Calculates one node's structural depth and rejects parent cycles.
///
/// # Parameters
/// - `plan`: Current plan projection.
/// - `node`: Stable node identity whose depth is requested.
///
/// # Returns
/// The number of `PlanNode` ancestors before the branch/root boundary.
pub(super) fn node_depth(plan: &Plan, node: EntityId) -> usize {
    let mut current = node;
    let mut visited = BTreeSet::new();
    let mut depth = 0;
    while visited.insert(current) {
        let Some(parent) = plan
            .nodes
            .iter()
            .find(|candidate| candidate.id == current)
            .and_then(|candidate| match candidate.parent {
                Some(PlanNodeParent::Node(parent)) => Some(parent),
                Some(PlanNodeParent::Branch(_)) | None => None,
            })
        else {
            break;
        };
        current = parent;
        depth += 1;
    }
    depth
}
