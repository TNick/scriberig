//! Verifies plan rollup and shared view behavior.

use super::*;

/// Keeps incremental status propagation equivalent to full reference rebuild.
#[test]
fn incremental_rollup_tracks_leaf_changes_and_reparenting() {
    let branch = EntityId::from_bytes([2; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([3; 16]),
        EntityId::from_bytes([1; 16]),
        ContinuationAnchor::BranchHead { branch },
        "incremental rollups",
    )
    .expect("plan");
    let mut parent = node(10, branch, "parent");
    parent.status = PlanNodeStatus::Running;
    plan.add_node(parent).expect("parent");
    let mut child = node(11, branch, "child");
    child.parent = Some(PlanNodeParent::Node(EntityId::from_bytes([10; 16])));
    plan.add_node(child).expect("child");
    let mut leaf = node(12, branch, "leaf");
    leaf.parent = Some(PlanNodeParent::Node(EntityId::from_bytes([11; 16])));
    plan.add_node(leaf).expect("leaf");
    plan.add_node(node(13, branch, "other root"))
        .expect("other root");
    let first_snapshot = scriberig_core::SnapshotId::new([1; 32]);
    let second_snapshot = scriberig_core::SnapshotId::new([2; 32]);
    let third_snapshot = scriberig_core::SnapshotId::new([3; 32]);
    let mut index = PlanNodeRollupIndex::rebuild(&plan, first_snapshot).expect("build index");
    let leaf_id = EntityId::from_bytes([12; 16]);
    plan.nodes
        .iter_mut()
        .find(|item| item.id == leaf_id)
        .expect("leaf node")
        .status = PlanNodeStatus::Failed;
    index
        .apply_change(&plan, first_snapshot, second_snapshot, &[leaf_id])
        .expect("propagate leaf change");
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );

    let moved_node = EntityId::from_bytes([11; 16]);
    let new_parent = EntityId::from_bytes([13; 16]);
    plan.nodes
        .iter_mut()
        .find(|item| item.id == moved_node)
        .expect("child node")
        .parent = Some(PlanNodeParent::Node(new_parent));
    index
        .apply_change(&plan, second_snapshot, third_snapshot, &[moved_node])
        .expect("propagate reparenting");
    assert_eq!(
        index.all(),
        &plan.node_rollups().expect("reference rollups")
    );
    assert_eq!(index.source_snapshot(), third_snapshot);
}

/// Refuses to reuse a derived index after its canonical base snapshot changes.
#[test]
fn incremental_rollup_rejects_a_stale_base_snapshot() {
    let plan = plan();
    let expected = scriberig_core::SnapshotId::new([1; 32]);
    let stale = scriberig_core::SnapshotId::new([2; 32]);
    let new = scriberig_core::SnapshotId::new([3; 32]);
    let mut index = PlanNodeRollupIndex::rebuild(&plan, expected).expect("index");

    assert_eq!(
        index.apply_change(&plan, stale, new, &[plan.nodes[0].id]),
        Err(scriberig_session::PlanError::StaleRollupIndex)
    );
    assert_eq!(index.source_snapshot(), expected);
}

/// Keeps derived descendant changes out of an ancestor's canonical revision.
#[test]
fn descendant_rollup_changes_do_not_revise_ancestor_content() {
    let branch = EntityId::from_bytes([2; 16]);
    let parent_id = EntityId::from_bytes([10; 16]);
    let child_id = EntityId::from_bytes([11; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([3; 16]),
        EntityId::from_bytes([1; 16]),
        ContinuationAnchor::BranchHead { branch },
        "derived rollup identity",
    )
    .expect("plan");
    let mut parent = node(10, branch, "parent");
    parent.status = PlanNodeStatus::Running;
    plan.add_node(parent).expect("parent");
    let mut child = node(11, branch, "child");
    child.parent = Some(PlanNodeParent::Node(parent_id));
    child.status = PlanNodeStatus::Completed;
    plan.add_node(child).expect("child");
    let rollup_before = plan.node_rollups().expect("rollups")[&parent_id];
    let before = PlanSemanticSnapshot::from_plan(&plan).expect("before snapshot");
    let parent_revision_before = before
        .objects
        .iter()
        .find(|object| object.entity == parent_id)
        .expect("parent revision")
        .revision;

    plan.nodes
        .iter_mut()
        .find(|item| item.id == child_id)
        .expect("child node")
        .status = PlanNodeStatus::Failed;

    let after = PlanSemanticSnapshot::from_plan(&plan).expect("after snapshot");
    let parent_revision_after = after
        .objects
        .iter()
        .find(|object| object.entity == parent_id)
        .expect("parent revision")
        .revision;
    assert_eq!(parent_revision_after, parent_revision_before);
    let rollup_after = plan.node_rollups().expect("updated rollups")[&parent_id];
    assert_ne!(rollup_after, rollup_before);
}

/// Matches incremental and reference rollups across a deep structural chain.
#[test]
fn incremental_rollup_matches_reference_for_a_deep_tree() {
    let branch = EntityId::from_bytes([2; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([3; 16]),
        EntityId::from_bytes([1; 16]),
        ContinuationAnchor::BranchHead { branch },
        "deep incremental rollup",
    )
    .expect("plan");
    let first_id = EntityId::from_bytes([20; 16]);
    let mut previous = first_id;
    for value in 20..=115 {
        let mut item = node(value, branch, "deep node");
        if value != 20 {
            item.parent = Some(PlanNodeParent::Node(previous));
        }
        previous = item.id;
        plan.add_node(item).expect("append deep node");
    }

    let first_snapshot = scriberig_core::SnapshotId::new([31; 32]);
    let next_snapshot = scriberig_core::SnapshotId::new([32; 32]);
    let mut index = PlanNodeRollupIndex::rebuild(&plan, first_snapshot).expect("index");
    plan.nodes
        .iter_mut()
        .find(|item| item.id == previous)
        .expect("leaf")
        .status = PlanNodeStatus::Failed;
    index
        .apply_change(&plan, first_snapshot, next_snapshot, &[previous])
        .expect("incremental leaf update");

    assert_eq!(index.all(), &plan.node_rollups().expect("reference"));
    assert_eq!(index.source_snapshot(), next_snapshot);
}

/// Produces shared exact-revision views with dependency blockers and counts.
#[test]
fn plan_views_expose_exact_revisions_and_shared_blockers() {
    let plan = plan();
    let snapshot = scriberig_core::SnapshotId::new([25; 32]);
    let plan_view = plan.plan_view(Some(snapshot)).expect("plan view");
    let rollups = plan.node_rollups().expect("rollups");
    let producer = EntityId::from_bytes([4; 16]);
    let consumer = EntityId::from_bytes([5; 16]);
    let consumer_view = plan
        .node_view(consumer, Some(snapshot), Some(&[]), &rollups)
        .expect("node view");
    let semantic = PlanSemanticSnapshot::from_plan(&plan).expect("semantic records");
    let plan_revision = semantic
        .objects
        .iter()
        .find(|object| object.entity == plan.id)
        .expect("plan revision");
    let node_revision = semantic
        .objects
        .iter()
        .find(|object| object.entity == consumer)
        .expect("node revision");

    assert_eq!(plan_view.revision, plan_revision.revision);
    assert_eq!(plan_view.snapshot, Some(snapshot));
    assert_eq!(plan_view.node_count, 2);
    assert_eq!(plan_view.status_counts.ready, 1);
    assert_eq!(plan_view.status_counts.planned, 1);
    assert_eq!(consumer_view.revision, node_revision.revision);
    assert_eq!(consumer_view.snapshot, Some(snapshot));
    assert_eq!(consumer_view.dependency_blockers, vec![producer]);
    assert_eq!(consumer_view.effective_status, PlanNodeStatus::Planned);
}

/// Builds the compact plan view without loading evidence or history objects.
#[test]
fn compact_semantic_plan_view_ignores_detailed_records() {
    let mut semantic = PlanSemanticSnapshot::from_plan(&plan()).expect("semantic records");
    semantic.snapshot = Some(scriberig_core::SnapshotId::new([40; 32]));
    semantic.objects.retain(|object| {
        matches!(
            object.object_type.as_str(),
            "scriberig.session.plan"
                | "scriberig.session.plan-node"
                | "scriberig.session.plan-membership"
                | "scriberig.session.plan-structure"
        )
    });

    let view = semantic.plan_view().expect("compact view");

    assert_eq!(view.id, semantic.plan);
    assert_eq!(view.snapshot, semantic.snapshot);
    assert_eq!(view.node_count, 2);
    assert_eq!(
        view.root_nodes,
        vec![EntityId::from_bytes([4; 16]), EntityId::from_bytes([5; 16])]
    );
    assert_eq!(view.status_counts.ready + view.status_counts.planned, 2);
}
