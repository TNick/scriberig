//! Verifies reference rollup precedence and deep incremental propagation.

use super::*;

/// Matches a deep leaf update to reference recomputation without changing a side tree.
#[test]
fn incremental_rollup_matches_reference_for_deep_tree_and_unrelated_root() {
    let branch = EntityId::from_bytes([240; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([250; 16]),
        EntityId::from_bytes([249; 16]),
        ContinuationAnchor::BranchHead { branch },
        "deep incremental rollup",
    )
    .expect("plan");
    for id in 1..=128 {
        let parent = if id == 1 {
            PlanNodeParent::Branch(branch)
        } else {
            PlanNodeParent::Node(EntityId::from_bytes([id - 1; 16]))
        };
        let mut current = node(id, branch, "deep node");
        current.parent = Some(parent);
        plan.add_node(current).expect("deep node");
    }
    let unrelated = EntityId::from_bytes([130; 16]);
    plan.add_node(node(130, branch, "unrelated root"))
        .expect("unrelated root");
    let original_snapshot = scriberig_core::SnapshotId::new([1; 32]);
    let updated_snapshot = scriberig_core::SnapshotId::new([2; 32]);
    let mut index = PlanNodeRollupIndex::rebuild(&plan, original_snapshot).expect("index");
    let unrelated_before = index.get(unrelated).copied().expect("unrelated rollup");
    let leaf = EntityId::from_bytes([128; 16]);
    plan.nodes
        .iter_mut()
        .find(|candidate| candidate.id == leaf)
        .expect("leaf")
        .status = PlanNodeStatus::Failed;

    index
        .apply_change(&plan, original_snapshot, updated_snapshot, &[leaf])
        .expect("incremental propagation");

    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );
    assert_eq!(index.get(unrelated), Some(&unrelated_before));
    assert_eq!(index.source_snapshot(), updated_snapshot);
}
