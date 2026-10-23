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

/// Compares incremental and reference rollups across status and ignore cases.
#[test]
fn incremental_rollup_matches_all_direct_status_combinations() {
    let branch = EntityId::from_bytes([241; 16]);
    let cases = [
        (
            PlanNodeStatus::Planned,
            PlanNodeStatus::Planned,
            PlanNodeStatus::Planned,
            false,
        ),
        (
            PlanNodeStatus::Running,
            PlanNodeStatus::Ready,
            PlanNodeStatus::Planned,
            false,
        ),
        (
            PlanNodeStatus::Running,
            PlanNodeStatus::Failed,
            PlanNodeStatus::Running,
            false,
        ),
        (
            PlanNodeStatus::Completed,
            PlanNodeStatus::Completed,
            PlanNodeStatus::Completed,
            false,
        ),
        (
            PlanNodeStatus::Completed,
            PlanNodeStatus::Abandoned,
            PlanNodeStatus::Completed,
            true,
        ),
    ];

    for (ordinal, (parent_status, child_status, sibling_status, ignored)) in
        cases.into_iter().enumerate()
    {
        let mut plan = Plan::new(
            EntityId::from_bytes([242; 16]),
            EntityId::from_bytes([243; 16]),
            ContinuationAnchor::BranchHead { branch },
            "status conformance",
        )
        .expect("plan");
        let parent_id = EntityId::from_bytes([1; 16]);
        let child_id = EntityId::from_bytes([2; 16]);
        let sibling_id = EntityId::from_bytes([3; 16]);
        let mut parent = node(1, branch, "parent");
        parent.status = parent_status;
        plan.add_node(parent).expect("parent");
        let mut child = node(2, branch, "child");
        child.parent = Some(PlanNodeParent::Node(parent_id));
        child.status = child_status;
        if ignored {
            child.abandon_reason = Some(scriberig_session::PlanAbandonReason::Ignored);
        }
        plan.add_node(child).expect("child");
        let mut sibling = node(3, branch, "sibling");
        sibling.parent = Some(PlanNodeParent::Node(parent_id));
        sibling.status = sibling_status;
        plan.add_node(sibling).expect("sibling");
        let identity = u8::try_from(ordinal).expect("case identity");
        let original = scriberig_core::SnapshotId::new([identity + 1; 32]);
        let updated = scriberig_core::SnapshotId::new([identity + 10; 32]);
        let mut index = PlanNodeRollupIndex::rebuild(&plan, original).expect("index");

        plan.nodes
            .iter_mut()
            .find(|candidate| candidate.id == child_id)
            .expect("child node")
            .status = PlanNodeStatus::Failed;
        index
            .apply_change(&plan, original, updated, &[child_id])
            .expect("incremental propagation");

        assert_eq!(
            index.all(),
            &plan.node_rollups().expect("reference rollups")
        );
        assert!(index.get(parent_id).is_some());
        assert!(index.get(sibling_id).is_some());
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the test compares all affected rollup cases in one fixture"
)]
/// Compares incremental rollups for ignored/abandoned and gated child changes.
#[test]
fn incremental_rollup_matches_ignored_gate_and_abandoned_cases() {
    let branch = EntityId::from_bytes([244; 16]);
    let parent_id = EntityId::from_bytes([1; 16]);
    let ignored_id = EntityId::from_bytes([2; 16]);
    let abandoned_id = EntityId::from_bytes([3; 16]);
    let gated_id = EntityId::from_bytes([4; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([245; 16]),
        EntityId::from_bytes([246; 16]),
        ContinuationAnchor::BranchHead { branch },
        "ignored rollup cases",
    )
    .expect("plan");
    let mut parent = node(1, branch, "parent");
    parent.status = PlanNodeStatus::Completed;
    plan.add_node(parent).expect("parent");
    let mut ignored = node(2, branch, "ignored child");
    ignored.parent = Some(PlanNodeParent::Node(parent_id));
    ignored.status = PlanNodeStatus::Abandoned;
    ignored.abandon_reason = Some(scriberig_session::PlanAbandonReason::Ignored);
    plan.add_node(ignored).expect("ignored child");
    let mut abandoned = node(3, branch, "cancelled child");
    abandoned.parent = Some(PlanNodeParent::Node(parent_id));
    abandoned.status = PlanNodeStatus::Abandoned;
    abandoned.abandon_reason = Some(scriberig_session::PlanAbandonReason::Cancelled);
    plan.add_node(abandoned).expect("abandoned child");
    let mut gated = node(4, branch, "required child");
    gated.parent = Some(PlanNodeParent::Node(parent_id));
    plan.add_node(gated).expect("required child");
    plan.add_gate(PlanGate {
        id: EntityId::from_bytes([5; 16]),
        owner: parent_id,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::RequiredChildCompleted { child: gated_id },
        waiver: None,
    })
    .expect("required completion gate");

    let initial = scriberig_core::SnapshotId::new([1; 32]);
    let mut index = PlanNodeRollupIndex::rebuild(&plan, initial).expect("index");
    let before_ignored = index.source_snapshot();
    let ignored_snapshot = scriberig_core::SnapshotId::new([7; 32]);
    plan.nodes
        .iter_mut()
        .find(|candidate| candidate.id == gated_id)
        .expect("gated child")
        .abandon_reason = Some(scriberig_session::PlanAbandonReason::Ignored);
    index
        .apply_change(&plan, before_ignored, ignored_snapshot, &[gated_id])
        .expect("required ignored child update");
    let required_ignored = index.get(parent_id).expect("parent rollup");
    assert!(required_ignored.contains_ignored);
    assert_eq!(required_ignored.unsatisfied_completion_gates, 1);
    assert_eq!(required_ignored.effective_status, PlanNodeStatus::Ready);
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );

    let waived_snapshot = scriberig_core::SnapshotId::new([8; 32]);
    plan.waive_gate(
        EntityId::from_bytes([5; 16]),
        PlanGateWaiver {
            reason: "reviewed ignored child".to_owned(),
            actor: None,
            operation: None,
        },
    )
    .expect("waive required child gate");
    index
        .apply_change(&plan, ignored_snapshot, waived_snapshot, &[parent_id])
        .expect("waiver update");
    let waived_parent = index.get(parent_id).expect("waived parent rollup");
    assert!(waived_parent.contains_ignored);
    assert_eq!(waived_parent.unsatisfied_completion_gates, 0);
    assert_eq!(waived_parent.effective_status, PlanNodeStatus::Ready);
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );

    for (node_id, status, ordinal) in [
        (ignored_id, PlanNodeStatus::Abandoned, 2),
        (abandoned_id, PlanNodeStatus::Abandoned, 3),
        (gated_id, PlanNodeStatus::Completed, 4),
    ] {
        let before = index.source_snapshot();
        let after = scriberig_core::SnapshotId::new([ordinal + 10; 32]);
        plan.nodes
            .iter_mut()
            .find(|candidate| candidate.id == node_id)
            .expect("target child")
            .status = status;
        index
            .apply_change(&plan, before, after, &[node_id])
            .expect("incremental update");
        assert_eq!(
            index.all(),
            &plan.node_rollups().expect("reference rollups")
        );
    }
    let parent = index.get(parent_id).expect("parent rollup");
    assert!(parent.contains_ignored);
    assert!(parent.contains_abandoned);
    assert_eq!(parent.effective_status, PlanNodeStatus::Completed);
}
