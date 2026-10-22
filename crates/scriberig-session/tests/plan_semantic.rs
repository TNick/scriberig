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
    assert_eq!(plan.ready_nodes_from_results(&[]), Vec::new());

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

#[path = "plan_semantic/persistence.rs"]
mod persistence;
#[path = "plan_semantic/rollup_reference.rs"]
mod rollup_reference;
#[path = "plan_semantic/rollups.rs"]
mod rollups;
