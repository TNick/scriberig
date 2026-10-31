//! Verifies semantic plan snapshot identity and history behavior.

use super::*;

/// Creates a valid semantic plan fixture with stable branch identities.
///
/// # Returns
/// A plan with two nodes and one explicit dependency.
pub(super) fn plan() -> Plan {
    let session = EntityId::from_bytes([1; 16]);
    let branch = EntityId::from_bytes([2; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([3; 16]),
        session,
        ContinuationAnchor::BranchHead { branch },
        "semantic plan fixture",
    )
    .expect("valid plan");
    plan.add_node(node(4, branch, "producer"))
        .expect("producer");
    plan.add_node(node(5, branch, "consumer"))
        .expect("consumer");
    plan.add_dependency(EntityId::from_bytes([4; 16]), EntityId::from_bytes([5; 16]))
        .expect("dependency");
    plan.transition(PlanStatus::Planned).expect("planned");
    plan
}

/// Round-trips the current plan graph through independent typed records.
#[test]
fn semantic_plan_snapshot_round_trips_plan_nodes_and_relations() {
    let expected = plan();
    let snapshot = PlanSemanticSnapshot::from_plan(&expected).expect("encode");
    let actual = snapshot
        .into_plan(expected.history.clone())
        .expect("assemble");
    let node_refs = snapshot.node_refs().expect("node references");

    assert_eq!(actual, expected);
    assert_eq!(node_refs.len(), expected.nodes.len());
    assert_eq!(node_refs[0].entity, expected.nodes[0].id);
    assert_eq!(node_refs[0].parent, expected.nodes[0].parent);
    assert_eq!(
        node_refs[0].revision,
        snapshot
            .objects
            .iter()
            .find(|object| object.entity == expected.nodes[0].id)
            .expect("node revision")
            .revision
    );
    assert_eq!(
        snapshot
            .direct_children(PlanNodeParent::Branch(expected.root_branch))
            .expect("direct children")
            .len(),
        expected.nodes.len()
    );
    assert!(
        snapshot
            .objects
            .iter()
            .any(|object| { object.object_type.as_str() == "scriberig.session.plan-membership" })
    );
    assert!(
        snapshot
            .objects
            .iter()
            .any(|object| { object.object_type.as_str() == "scriberig.session.plan-structure" })
    );
    assert!(
        snapshot
            .objects
            .iter()
            .any(|object| { object.object_type.as_str() == "scriberig.session.plan-dependency" })
    );
}

/// Keeps a plan revision stable when only one node payload changes.
#[test]
fn semantic_plan_node_revision_is_independent_from_plan_revision() {
    let original = plan();
    let mut changed = original.clone();
    let node_id = changed.nodes[0].id;
    changed.nodes[0].title.push_str(" revised");
    changed.nodes[0].status = PlanNodeStatus::Ready;

    let before = PlanSemanticSnapshot::from_plan(&original).expect("encode original");
    let after = PlanSemanticSnapshot::from_plan(&changed).expect("encode changed");
    let original_plan = before
        .objects
        .iter()
        .find(|object| object.entity == original.id)
        .expect("plan record");
    let changed_plan = after
        .objects
        .iter()
        .find(|object| object.entity == changed.id)
        .expect("changed plan record");
    let original_node = before
        .objects
        .iter()
        .find(|object| object.entity == node_id)
        .expect("node record");
    let changed_node = after
        .objects
        .iter()
        .find(|object| object.entity == node_id)
        .expect("changed node record");

    assert_eq!(original_plan.revision, changed_plan.revision);
    assert_ne!(original_node.revision, changed_node.revision);
}

/// Limits worker-claim revision churn to the plan and claimed attempt records.
#[test]
fn semantic_worker_claim_preserves_unrelated_node_and_relation_revisions() {
    let mut plan = plan();
    let claimed_node = plan.nodes[0].id;
    let sibling_node = plan.nodes[1].id;
    plan.start_with_coordinator_claim(EntityId::from_bytes([86; 16]), "coordinator", None)
        .expect("start plan");
    let before = PlanSemanticSnapshot::from_plan(&plan).expect("encode ready plan");
    let mutation_revision = plan.revision;

    plan.start_node(
        claimed_node,
        EntityId::from_bytes([87; 16]),
        EntityId::from_bytes([88; 16]),
        "worker",
        &[],
    )
    .expect("claim ready node");
    let after = PlanSemanticSnapshot::from_plan(&plan).expect("encode claimed plan");

    assert_eq!(plan.revision, mutation_revision + 1);
    let changed_objects = after
        .objects
        .iter()
        .filter(|current| {
            before
                .objects
                .iter()
                .find(|prior| prior.entity == current.entity)
                .is_none_or(|prior| prior.revision != current.revision)
        })
        .map(|object| (object.entity, object.object_type.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(changed_objects.len(), 4);
    assert!(changed_objects.contains(&(plan.id, "scriberig.session.plan")));
    assert!(changed_objects.contains(&(claimed_node, "scriberig.session.plan-node")));
    let attempt = after
        .attempts()
        .expect("attempt records")
        .into_iter()
        .next()
        .expect("new worker attempt");
    assert!(changed_objects.contains(&(attempt.entity, "scriberig.session.plan-attempt")));
    assert_eq!(
        changed_objects
            .iter()
            .filter(|(_, object_type)| *object_type == "scriberig.session.plan-revision")
            .count(),
        1
    );
    let prior_sibling = before
        .objects
        .iter()
        .find(|object| object.entity == sibling_node)
        .expect("prior sibling revision");
    let current_sibling = after
        .objects
        .iter()
        .find(|object| object.entity == sibling_node)
        .expect("current sibling revision");
    assert_eq!(prior_sibling.revision, current_sibling.revision);
    for object_type in [
        "scriberig.session.plan-membership",
        "scriberig.session.plan-structure",
        "scriberig.session.plan-dependency",
    ] {
        let prior = before
            .objects
            .iter()
            .filter(|object| object.object_type.as_str() == object_type)
            .map(|object| (object.entity, object.revision))
            .collect::<Vec<_>>();
        let current = after
            .objects
            .iter()
            .filter(|object| object.object_type.as_str() == object_type)
            .map(|object| (object.entity, object.revision))
            .collect::<Vec<_>>();
        assert_eq!(prior, current, "unexpected {object_type} revision churn");
    }
}

/// Keeps each worker attempt independently identified across status revisions.
#[test]
fn semantic_plan_attempt_has_stable_identity_and_exact_revision() {
    let mut running = plan();
    let node = running.nodes[0].id;
    let branch = EntityId::from_bytes([81; 16]);
    let claim = EntityId::from_bytes([82; 16]);
    running
        .start_with_coordinator_claim(EntityId::from_bytes([83; 16]), "coordinator", None)
        .expect("start plan");
    running
        .start_node(node, branch, claim, "worker", &[])
        .expect("start worker attempt");
    let running_attempt = PlanSemanticSnapshot::from_plan(&running)
        .expect("encode running plan")
        .attempts()
        .expect("read attempt")
        .into_iter()
        .next()
        .expect("running attempt exists");

    let mut failed = running.clone();
    failed
        .fail_node(node, claim)
        .expect("settle failed attempt");
    let failed_attempt = PlanSemanticSnapshot::from_plan(&failed)
        .expect("encode failed plan")
        .attempts()
        .expect("read attempt")
        .into_iter()
        .next()
        .expect("failed attempt exists");

    assert_eq!(running_attempt.entity, failed_attempt.entity);
    assert_ne!(running_attempt.revision, failed_attempt.revision);
    assert_eq!(running_attempt.node, node);
    assert_eq!(running_attempt.branch, branch);
    assert_eq!(running_attempt.claim, Some(claim));
    assert_eq!(running_attempt.status, PlanNodeStatus::Running);
    assert_eq!(failed_attempt.status, PlanNodeStatus::Failed);
}

/// Changes structural parentage without revising the node content object.
#[test]
fn semantic_reparenting_revises_structure_without_revising_node_content() {
    let original = plan();
    let child = original.nodes[0].id;
    let new_parent = original.nodes[1].id;
    let mut moved = original.clone();
    moved.nodes[0].parent = Some(PlanNodeParent::Node(new_parent));

    let before = PlanSemanticSnapshot::from_plan(&original).expect("encode original");
    let after = PlanSemanticSnapshot::from_plan(&moved).expect("encode moved");
    let before_node = before
        .objects
        .iter()
        .find(|object| object.entity == child)
        .expect("node revision");
    let after_node = after
        .objects
        .iter()
        .find(|object| object.entity == child)
        .expect("node revision");
    let before_structures = before
        .objects
        .iter()
        .filter(|object| object.object_type.as_str() == "scriberig.session.plan-structure")
        .map(|object| object.revision)
        .collect::<Vec<_>>();
    let after_structures = after
        .objects
        .iter()
        .filter(|object| object.object_type.as_str() == "scriberig.session.plan-structure")
        .map(|object| object.revision)
        .collect::<Vec<_>>();

    assert_eq!(before_node.revision, after_node.revision);
    assert_ne!(before_structures, after_structures);
}

/// Rejects modified object bytes whose recorded content revision is stale.
#[test]
fn semantic_plan_snapshot_validates_revision_identity_before_assembly() {
    let mut snapshot = PlanSemanticSnapshot::from_plan(&plan()).expect("encode");
    snapshot.objects[0].payload.push(0);

    assert!(snapshot.into_plan(Vec::new()).is_err());
}

/// Stores mutation reasons outside the compact plan revision payload.
#[test]
fn semantic_plan_history_metadata_round_trips_independently() {
    let mut expected = plan();
    expected.history.last_mut().expect("current history").reason =
        Some("authorized replan".to_owned());
    let snapshot = PlanSemanticSnapshot::from_plan(&expected).expect("encode");
    let metadata = snapshot.history_metadata().expect("history metadata");
    let mut projection = snapshot
        .into_plan(expected.history.clone())
        .expect("assemble");

    assert!(metadata.iter().any(|item| {
        item.mutation_revision == expected.revision
            && item.reason.as_deref() == Some("authorized replan")
    }));
    projection
        .history
        .last_mut()
        .expect("current history")
        .reason = None;
    assert_eq!(
        snapshot
            .into_plan(projection.history)
            .expect("assemble from stored metadata")
            .history
            .last()
            .and_then(|record| record.reason.as_deref()),
        Some("authorized replan")
    );
}
