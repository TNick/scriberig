//! Compares incremental plan rollups with the reference evaluator.

use super::*;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The table-like scenario exercises each required rollup transition."
)]
/// Compares incremental rollups across node, gate, and sibling changes.
fn incremental_rollup_matches_reference_for_status_gate_and_sibling_cases() {
    let branch = EntityId::from_bytes([2; 16]);
    let parent_id = EntityId::from_bytes([30; 16]);
    let worker_id = EntityId::from_bytes([31; 16]);
    let ignored_id = EntityId::from_bytes([32; 16]);
    let other_root_id = EntityId::from_bytes([33; 16]);
    let gate_id = EntityId::from_bytes([34; 16]);
    let abandoned_root_id = EntityId::from_bytes([37; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([35; 16]),
        EntityId::from_bytes([1; 16]),
        ContinuationAnchor::BranchHead { branch },
        "incremental rollup conformance",
    )
    .expect("plan");
    let parent = node(30, branch, "parent");
    plan.add_node(parent).expect("parent");
    let mut worker = node(31, branch, "worker");
    worker.parent = Some(PlanNodeParent::Node(parent_id));
    plan.add_node(worker).expect("worker");
    let mut ignored = node(32, branch, "ignored sibling");
    ignored.parent = Some(PlanNodeParent::Node(parent_id));
    plan.add_node(ignored).expect("ignored sibling");
    plan.add_node(node(33, branch, "unrelated root"))
        .expect("unrelated root");
    plan.add_node(node(37, branch, "abandoned-only root"))
        .expect("abandoned-only root");
    let mut abandoned = node(36, branch, "superseded child");
    abandoned.parent = Some(PlanNodeParent::Node(abandoned_root_id));
    abandoned.status = PlanNodeStatus::Abandoned;
    abandoned.abandon_reason = Some(scriberig_session::PlanAbandonReason::Superseded);
    plan.add_node(abandoned).expect("superseded child");
    for node_id in [parent_id, worker_id, ignored_id] {
        plan.nodes
            .iter_mut()
            .find(|candidate| candidate.id == node_id)
            .expect("planned subtree node")
            .status = PlanNodeStatus::Planned;
    }

    let first = scriberig_core::SnapshotId::new([50; 32]);
    let mut index = PlanNodeRollupIndex::rebuild(&plan, first).expect("initial index");
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("initial reference")
    );
    assert_eq!(plan.nodes[1].parent, Some(PlanNodeParent::Node(parent_id)));
    assert_eq!(plan.nodes[2].parent, Some(PlanNodeParent::Node(parent_id)));
    assert_eq!(index.get(parent_id).unwrap().child_counts.planned, 2);
    let unrelated_before = index.get(other_root_id).copied();
    let mut snapshot = first;
    let statuses = [
        PlanNodeStatus::Ready,
        PlanNodeStatus::Running,
        PlanNodeStatus::Failed,
        PlanNodeStatus::Ready,
        PlanNodeStatus::Running,
        PlanNodeStatus::Completed,
    ];
    for (step, status) in statuses.into_iter().enumerate() {
        plan.nodes
            .iter_mut()
            .find(|candidate| candidate.id == worker_id)
            .expect("worker node")
            .status = status;
        let step = u8::try_from(step).expect("status count fits in u8");
        let next = scriberig_core::SnapshotId::new([51 + step; 32]);
        index
            .apply_change(&plan, snapshot, next, &[worker_id])
            .expect("apply status change");
        assert_eq!(
            index.all(),
            &plan.node_rollups().expect("reference rollups")
        );
        assert_eq!(index.get(other_root_id).copied(), unrelated_before);
        snapshot = next;
    }

    plan.nodes
        .iter_mut()
        .find(|candidate| candidate.id == ignored_id)
        .expect("second child")
        .status = PlanNodeStatus::Completed;
    let next = scriberig_core::SnapshotId::new([57; 32]);
    index
        .apply_change(&plan, snapshot, next, &[ignored_id])
        .expect("apply completed sibling");
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );
    snapshot = next;

    plan.nodes
        .iter_mut()
        .find(|candidate| candidate.id == worker_id)
        .expect("worker node")
        .status = PlanNodeStatus::Failed;
    plan.nodes
        .iter_mut()
        .find(|candidate| candidate.id == ignored_id)
        .expect("ignored sibling")
        .status = PlanNodeStatus::Running;
    let next = scriberig_core::SnapshotId::new([58; 32]);
    index
        .apply_change(&plan, snapshot, next, &[worker_id, ignored_id])
        .expect("apply failed and running siblings");
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );
    snapshot = next;

    let ignored = plan
        .nodes
        .iter_mut()
        .find(|candidate| candidate.id == ignored_id)
        .expect("ignored node");
    ignored.status = PlanNodeStatus::Abandoned;
    ignored.abandon_reason = Some(scriberig_session::PlanAbandonReason::Ignored);
    let next = scriberig_core::SnapshotId::new([59; 32]);
    index
        .apply_change(&plan, snapshot, next, &[ignored_id])
        .expect("apply ignored status");
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );
    snapshot = next;

    plan.nodes
        .iter_mut()
        .find(|candidate| candidate.id == parent_id)
        .expect("parent node")
        .status = PlanNodeStatus::Completed;
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: parent_id,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::RequiredChildCompleted { child: ignored_id },
        waiver: None,
    })
    .expect("required ignored child gate");
    let next = scriberig_core::SnapshotId::new([60; 32]);
    index
        .apply_change(&plan, snapshot, next, &[parent_id])
        .expect("apply completion gate");
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );
    assert_eq!(
        index.get(parent_id).unwrap().effective_status,
        PlanNodeStatus::Planned
    );
    assert_eq!(
        index.get(parent_id).unwrap().unsatisfied_completion_gates,
        1
    );
    snapshot = next;

    plan.waive_gate(
        gate_id,
        PlanGateWaiver {
            reason: "explicitly accepted ignored work".to_owned(),
            actor: None,
            operation: None,
        },
    )
    .expect("waive completion gate");
    let next = scriberig_core::SnapshotId::new([61; 32]);
    index
        .apply_change(&plan, snapshot, next, &[parent_id])
        .expect("apply gate waiver");
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );
    assert_eq!(
        index.get(parent_id).unwrap().effective_status,
        PlanNodeStatus::Completed
    );
    assert!(index.get(parent_id).unwrap().contains_ignored);
    assert!(index.get(parent_id).unwrap().contains_abandoned);
    assert!(!index.get(abandoned_root_id).unwrap().contains_ignored);
    assert!(index.get(abandoned_root_id).unwrap().contains_abandoned);
}
