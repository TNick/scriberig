//! Verifies bounded shared plan and node view projections.

use super::*;

/// Preserves exact revisions, snapshots, and dependency blockers in views.
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
