//! Verifies immutable semantic plan object encoding and assembly.

use scriberig_core::EntityId;
use scriberig_session::{
    ContinuationAnchor, Plan, PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner,
    PlanEvidenceReference, PlanEvidenceRole, PlanGate, PlanGatePhase, PlanGateRequirement,
    PlanGateStatus, PlanGateWaiver, PlanNode, PlanNodeParent, PlanNodeRollupIndex, PlanNodeStatus,
    PlanNodeSummary, PlanSemanticReference, PlanSemanticSnapshot, PlanStatus,
};

/// Creates a valid unstarted plan node with a stable identity.
///
/// # Parameters
/// - `id`: Stable node identity byte repeated across the ID.
/// - `branch`: Existing branch used as the structural parent.
/// - `title`: Human-readable node title and purpose.
///
/// # Returns
/// A planned node with no execution attempt.
fn node(id: u8, branch: EntityId, title: &str) -> PlanNode {
    PlanNode {
        id: EntityId::from_bytes([id; 16]),
        parent: Some(PlanNodeParent::Branch(branch)),
        title: title.to_owned(),
        purpose: title.to_owned(),
        expected_result: None,
        semantic_scope: Vec::new(),
        status: PlanNodeStatus::Planned,
        branch: None,
        result: None,
        superseded_by: None,
        abandon_reason: None,
        last_failure: None,
        assignment: None,
        claim: None,
        claim_generation: 0,
    }
}

/// Round-trips independent gate, evidence, and summary records.
#[test]
fn semantic_plan_requirement_records_round_trip_and_block_entry() {
    let branch = EntityId::from_bytes([2; 16]);
    let node_id = EntityId::from_bytes([4; 16]);
    let gate_id = EntityId::from_bytes([6; 16]);
    let evidence_id = EntityId::from_bytes([7; 16]);
    let summary_id = EntityId::from_bytes([8; 16]);
    let mut plan = Plan::new(
        EntityId::from_bytes([3; 16]),
        EntityId::from_bytes([1; 16]),
        ContinuationAnchor::BranchHead { branch },
        "gate round trip",
    )
    .expect("valid plan");
    plan.add_node(node(4, branch, "validated step"))
        .expect("node");
    plan.transition(PlanStatus::Planned).expect("planned");
    plan.transition(PlanStatus::Running).expect("running");
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: node_id,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::ValidationPassed {
            profile: Some("lint".to_owned()),
        },
        waiver: None,
    })
    .expect("gate");

    assert_eq!(plan.nodes[0].status, PlanNodeStatus::Planned);
    assert_eq!(
        plan.evaluate_gate(gate_id).expect("evaluation").status,
        PlanGateStatus::Pending
    );
    assert!(plan.ready_nodes_from_results(&[]).is_empty());

    let mut attributes = std::collections::BTreeMap::new();
    attributes.insert("passed".to_owned(), "true".to_owned());
    attributes.insert("profile".to_owned(), "lint".to_owned());
    plan.attach_evidence(PlanEvidenceAssociation {
        id: evidence_id,
        owner: PlanEvidenceOwner::Gate(gate_id),
        role: PlanEvidenceRole::Supports,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::ValidationResult,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([9; 16]),
                revision: None,
                snapshot: None,
            },
            attributes,
        },
        note: Some("lint passed".to_owned()),
        actor: None,
    })
    .expect("evidence");
    plan.update_summary(PlanNodeSummary {
        id: summary_id,
        node: node_id,
        text: "Validation is complete".to_owned(),
        sources: vec![PlanSemanticReference {
            entity: EntityId::from_bytes([9; 16]),
            revision: None,
            snapshot: None,
        }],
    })
    .expect("summary");

    assert_eq!(plan.nodes[0].status, PlanNodeStatus::Ready);
    assert_eq!(plan.ready_nodes_from_results(&[]), vec![node_id]);
    assert_eq!(
        plan.evaluate_gate(gate_id).expect("evaluation").status,
        PlanGateStatus::Satisfied
    );
    let snapshot = PlanSemanticSnapshot::from_plan(&plan).expect("encode");
    let decoded = snapshot.into_plan(plan.history.clone()).expect("decode");

    assert_eq!(decoded, plan);
}

/// Derives effective parent status and ignored/abandoned descendant flags.
#[test]
fn plan_rollup_respects_required_abandoned_child_and_waiver() {
    let mut plan = plan();
    let parent = EntityId::from_bytes([10; 16]);
    let child = EntityId::from_bytes([11; 16]);
    let gate = EntityId::from_bytes([12; 16]);
    let branch = plan.root_branch;
    plan.add_node(node(10, branch, "parent")).expect("parent");
    plan.nodes
        .iter_mut()
        .find(|node| node.id == parent)
        .expect("parent node")
        .status = PlanNodeStatus::Completed;
    let mut child_node = node(11, branch, "ignored child");
    child_node.parent = Some(PlanNodeParent::Node(parent));
    child_node.status = PlanNodeStatus::Abandoned;
    child_node.abandon_reason = Some(scriberig_session::PlanAbandonReason::Ignored);
    plan.add_node(child_node).expect("child");
    plan.add_gate(PlanGate {
        id: gate,
        owner: parent,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::RequiredChildCompleted { child },
        waiver: None,
    })
    .expect("required child gate");

    let rollup = plan.node_rollups().expect("rollups")[&parent];
    assert_eq!(rollup.effective_status, PlanNodeStatus::Planned);
    assert!(rollup.contains_ignored);
    assert!(rollup.contains_abandoned);
    assert_eq!(rollup.unsatisfied_completion_gates, 1);

    plan.waive_gate(
        gate,
        PlanGateWaiver {
            reason: "ignored after review".to_owned(),
            actor: None,
            operation: None,
        },
    )
    .expect("waiver");
    let waived = plan.node_rollups().expect("waived rollups")[&parent];
    assert_eq!(waived.effective_status, PlanNodeStatus::Completed);
    assert_eq!(waived.unsatisfied_completion_gates, 0);
    assert!(waived.contains_ignored);
}

/// Rejects malformed structural cycles before deriving effective status.
#[test]
fn plan_rollup_rejects_structural_cycles() {
    let mut plan = plan();
    let first = plan.nodes[0].id;
    let second = plan.nodes[1].id;
    plan.nodes[0].parent = Some(PlanNodeParent::Node(second));
    plan.nodes[1].parent = Some(PlanNodeParent::Node(first));

    assert_eq!(
        plan.node_rollups(),
        Err(scriberig_session::PlanError::StructuralCycle)
    );
}

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

/// Creates a plan with stable node, structure, and dependency identities.
///
/// # Returns
/// A plan with two nodes and one explicit dependency.
fn plan() -> Plan {
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
