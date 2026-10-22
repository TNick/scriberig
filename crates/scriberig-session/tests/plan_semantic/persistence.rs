//! Verifies independent semantic plan storage and revision behavior.

use super::*;

/// Verifies typed plan identities and payload revisions are deterministic.
#[test]
fn semantic_encoding_is_deterministic_and_uses_stable_types() {
    let mut plan = plan();
    let node = plan.nodes[0].id;
    plan.coordinator = Some(plan.root_branch);
    plan.coordinator_cursor = Some(node);
    plan.coordinator_generation = 3;
    plan.coordinator_claim = Some(scriberig_session::CoordinatorClaim {
        id: EntityId::from_bytes([10; 16]),
        operation: None,
        owner: "runtime-a".to_owned(),
        generation: 3,
    });
    let gate = EntityId::from_bytes([11; 16]);
    plan.add_gate(PlanGate {
        id: gate,
        owner: node,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::Artifact,
        },
        waiver: None,
    })
    .expect("add gate");
    plan.attach_evidence(PlanEvidenceAssociation {
        id: EntityId::from_bytes([12; 16]),
        owner: PlanEvidenceOwner::Gate(gate),
        role: PlanEvidenceRole::Supports,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::Artifact,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([13; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::new(),
        },
        note: None,
        actor: None,
    })
    .expect("attach evidence");
    plan.update_summary(PlanNodeSummary {
        id: EntityId::from_bytes([14; 16]),
        node,
        text: "summary with exact provenance references".to_owned(),
        sources: vec![PlanSemanticReference {
            entity: EntityId::from_bytes([15; 16]),
            revision: None,
            snapshot: None,
        }],
    })
    .expect("set summary");

    let first = PlanSemanticSnapshot::from_plan(&plan).expect("first encoding");
    let second = PlanSemanticSnapshot::from_plan(&plan).expect("second encoding");
    let types = first
        .objects
        .iter()
        .map(|object| object.object_type.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let plan_object = first
        .objects
        .iter()
        .find(|object| object.entity == plan.id)
        .expect("independent plan object");
    let plan_payload: serde_json::Value =
        serde_json::from_slice(&plan_object.payload).expect("plan payload");
    let node_object = first
        .objects
        .iter()
        .find(|object| object.entity == node)
        .expect("independent node object");
    let node_payload: serde_json::Value =
        serde_json::from_slice(&node_object.payload).expect("node payload");
    let restored = first
        .into_plan(plan.history.clone())
        .expect("round-trip plan and coordinator");
    let expected = [
        "scriberig.session.plan",
        "scriberig.session.plan-node",
        "scriberig.session.plan-membership",
        "scriberig.session.plan-structure",
        "scriberig.session.plan-dependency",
        "scriberig.session.plan-revision",
        "scriberig.session.plan-gate",
        "scriberig.session.plan-evidence",
        "scriberig.session.plan-summary",
        "scriberig.session.plan-summary-link",
    ];

    assert_eq!(first, second);
    assert!(expected.into_iter().all(|item| types.contains(item)));
    assert_eq!(restored, plan);
    assert!(plan_payload.get("nodes").is_none());
    assert!(plan_payload.get("dependencies").is_none());
    assert!(plan_payload.get("history").is_none());
    assert!(node_payload.get("effective_status").is_none());
    assert!(node_payload.get("status_counts").is_none());
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
