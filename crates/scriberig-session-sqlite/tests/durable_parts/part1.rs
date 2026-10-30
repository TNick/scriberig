//! Covers additional focused cases for the parent test module.

use super::*;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The persistence regression verifies one canonical graph across reload."
)]
/// Confirms plan graphs are split across universal immutable revisions.
fn plan_storage_uses_independent_universal_objects() {
    use scriberig_core::RootName;
    use scriberig_store::{RevisionStore, RootStore, SnapshotStore};

    let path = std::env::temp_dir().join(format!(
        "scriberig-semantic-plan-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([51; 16]);
    let root_branch = EntityId::from_bytes([52; 16]);
    let plan_id = EntityId::from_bytes([53; 16]);
    let node_id = EntityId::from_bytes([54; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead {
            branch: root_branch,
        },
        "persist independent plan objects",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(root_branch)),
        title: "inspect graph".to_owned(),
        purpose: "verify semantic object ownership".to_owned(),
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
    })
    .expect("add node");
    let gate_id = EntityId::from_bytes([55; 16]);
    plan.add_gate(scriberig_session::PlanGate {
        id: gate_id,
        owner: node_id,
        phase: scriberig_session::PlanGatePhase::Entry,
        requirement: scriberig_session::PlanGateRequirement::EvidencePresent {
            kind: scriberig_session::PlanEvidenceKind::Artifact,
        },
        waiver: None,
    })
    .expect("add gate");
    plan.attach_evidence(scriberig_session::PlanEvidenceAssociation {
        id: EntityId::from_bytes([56; 16]),
        owner: scriberig_session::PlanEvidenceOwner::Gate(gate_id),
        role: scriberig_session::PlanEvidenceRole::Supports,
        evidence: scriberig_session::PlanEvidenceReference {
            kind: scriberig_session::PlanEvidenceKind::Artifact,
            reference: scriberig_session::PlanSemanticReference {
                entity: EntityId::from_bytes([58; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::default(),
        },
        note: None,
        actor: None,
    })
    .expect("attach evidence");
    plan.update_summary(scriberig_session::PlanNodeSummary {
        id: EntityId::from_bytes([57; 16]),
        node: node_id,
        text: "artifact is available".to_owned(),
        sources: Vec::new(),
    })
    .expect("update summary");
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_branch, "root").expect("branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save session");

    let semantic_store =
        scriberig_store_sqlite::SqliteStore::open(&path).expect("open universal store");
    let root =
        RootName::new(format!("scriberig-plan-{session_id}-{plan_id}")).expect("plan root name");
    let snapshot = semantic_store
        .read_root(&root)
        .expect("canonical plan root exists");
    let mappings = semantic_store.iter(snapshot).expect("plan objects");
    let plan_revision_id = mappings
        .iter()
        .find_map(|(entity, revision)| (*entity == plan_id).then_some(*revision))
        .expect("plan object mapping");
    let node_revision_id = mappings
        .iter()
        .find_map(|(entity, revision)| (*entity == node_id).then_some(*revision))
        .expect("independent node mapping");
    let plan_revision = semantic_store
        .get(plan_revision_id)
        .expect("read plan revision")
        .expect("plan revision exists");
    let node_revision = semantic_store
        .get(node_revision_id)
        .expect("read node revision")
        .expect("node revision exists");
    let semantic_objects = mappings
        .iter()
        .map(|(entity, revision_id)| {
            let revision = semantic_store
                .get(*revision_id)
                .expect("read semantic revision")
                .expect("semantic revision exists");
            scriberig_session::PlanSemanticRevision {
                entity: *entity,
                object_type: revision.object_type().clone(),
                revision: revision.id(),
                payload: revision.payload().to_vec(),
            }
        })
        .collect();
    let semantic_snapshot = scriberig_session::PlanSemanticSnapshot {
        plan: plan_id,
        snapshot: Some(snapshot),
        objects: semantic_objects,
    };
    let plan_payload: serde_json::Value =
        serde_json::from_slice(plan_revision.payload()).expect("plan payload JSON");

    assert_eq!(
        plan_revision.object_type().as_str(),
        "scriberig.session.plan"
    );
    assert_eq!(
        node_revision.object_type().as_str(),
        "scriberig.session.plan-node"
    );
    assert!(plan_payload.get("nodes").is_none());
    assert!(plan_payload.get("dependencies").is_none());
    assert!(plan_payload.get("history").is_none());
    assert!(mappings.len() > 2);
    let exact_nodes = semantic_snapshot.node_refs().expect("exact node refs");
    assert_eq!(exact_nodes.len(), 1);
    assert_eq!(exact_nodes[0].entity, node_id);
    assert_eq!(exact_nodes[0].revision, node_revision_id);
    let decoded = semantic_snapshot
        .into_plan(Vec::new())
        .expect("decode plan with gate, evidence, and summary");
    assert_eq!(decoded.gates.len(), 1);
    assert_eq!(decoded.evidence.len(), 1);
    assert_eq!(decoded.summaries.len(), 1);
    assert_eq!(
        decoded
            .evaluate_gate(gate_id)
            .expect("evaluate gate")
            .status,
        scriberig_session::PlanGateStatus::Satisfied
    );
    drop(semantic_store);
    drop(store);
    let _ = std::fs::remove_file(path);
}

/// Verifies plan compare and swap merges new worker branch.
#[test]
fn plan_compare_and_swap_merges_new_worker_branch() {
    let session_id = EntityId::from_bytes([31; 16]);
    let root_id = EntityId::from_bytes([32; 16]);
    let worker_id = EntityId::from_bytes([33; 16]);
    let plan_id = EntityId::from_bytes([34; 16]);
    let store = SqliteSessionStore::open_in_memory().expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "run worker",
    )
    .expect("plan");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("planned");
    let initial_revision = plan.revision;
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial graph");
    let mut proposed = store
        .load(session_id)
        .expect("load graph")
        .expect("stored graph");
    proposed.plans[0]
        .transition(scriberig_session::PlanStatus::Running)
        .expect("start plan");
    proposed.branches.push(
        Branch::new_child(worker_id, root_id, "worker", "perform work").expect("worker branch"),
    );
    proposed.branches[0].children.push(worker_id);

    store
        .compare_and_swap_plan(&proposed, plan_id, initial_revision)
        .expect("atomically persist plan and worker branch");

    let stored = store
        .load(session_id)
        .expect("reload graph")
        .expect("stored graph");
    assert_eq!(
        stored.plans[0].status,
        scriberig_session::PlanStatus::Running
    );
    assert_eq!(stored.branches.len(), 2);
    assert_eq!(stored.branches[0].children, vec![worker_id]);
    assert_eq!(stored.branches[1].parent, Some(root_id));
}

/// Resolves exact historical plan membership without consulting stale payloads.
#[test]
fn plan_snapshot_reads_preserve_historical_membership() {
    let session_id = EntityId::from_bytes([61; 16]);
    let root_id = EntityId::from_bytes([62; 16]);
    let plan_id = EntityId::from_bytes([63; 16]);
    let node_id = EntityId::from_bytes([64; 16]);
    let store = SqliteSessionStore::open_in_memory().expect("open store");
    let plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "snapshot membership",
    )
    .expect("plan");
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial plan");

    let mut proposal = store
        .load(session_id)
        .expect("load session")
        .expect("stored session");
    let source_snapshot = proposal.plans[0]
        .source_snapshot
        .expect("loaded plan snapshot");
    proposal.plans[0]
        .add_node(PlanNode {
            id: node_id,
            parent: Some(scriberig_session::PlanNodeParent::Branch(root_id)),
            title: "new work".to_owned(),
            purpose: "verify exact membership".to_owned(),
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
        })
        .expect("add node");
    store
        .compare_and_swap_plan(&proposal, plan_id, 0)
        .expect("promote plan mutation");

    let historical = store
        .load_plan_at_snapshot(session_id, plan_id, source_snapshot)
        .expect("read historical snapshot")
        .expect("snapshot belongs to plan history");
    let current = store
        .load(session_id)
        .expect("reload session")
        .expect("stored session");

    assert_eq!(historical.nodes, Vec::new());
    assert_eq!(historical.source_snapshot, Some(source_snapshot));
    assert_eq!(current.plans[0].nodes.len(), 1);
    assert_eq!(current.plans[0].nodes[0].id, node_id);
    assert_eq!(
        store
            .load_plan_at_snapshot(
                session_id,
                plan_id,
                scriberig_core::SnapshotId::new([255; 32]),
            )
            .expect("foreign snapshot lookup"),
        None
    );
}

/// Rolls back branch persistence when semantic plan promotion fails.
#[test]
fn plan_cas_rolls_back_session_projection_on_semantic_conflict() {
    let path =
        std::env::temp_dir().join(format!("scriberig-plan-atomic-{}.sqlite", EntityId::new()));
    let session_id = EntityId::from_bytes([65; 16]);
    let root_id = EntityId::from_bytes([66; 16]);
    let worker_id = EntityId::from_bytes([67; 16]);
    let plan_id = EntityId::from_bytes([68; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "atomic plan mutation",
    )
    .expect("plan");
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save session");
    let mut proposal = store
        .load(session_id)
        .expect("load session")
        .expect("stored session");
    proposal.plans[0]
        .transition(scriberig_session::PlanStatus::Planned)
        .expect("plan plan");
    let expected = 0;
    proposal
        .branches
        .push(Branch::new_child(worker_id, root_id, "worker", "perform work").expect("worker"));
    proposal.branches[0].children.push(worker_id);
    let connection = rusqlite::Connection::open(&path).expect("open trigger connection");
    connection
        .execute_batch(
            "CREATE TRIGGER reject_plan_root_update
             BEFORE UPDATE ON roots
             BEGIN SELECT RAISE(ABORT, 'injected promotion failure'); END;",
        )
        .expect("install root promotion failure");
    drop(connection);

    assert!(
        store
            .compare_and_swap_plan(&proposal, plan_id, expected)
            .is_err()
    );
    let stored = store
        .load(session_id)
        .expect("reload session")
        .expect("stored session");

    assert_eq!(stored.branches.len(), 1);
    assert_eq!(stored.plans[0].status, scriberig_session::PlanStatus::Draft);
    drop(store);
    let _ = std::fs::remove_file(path);
}

#[path = "part1/selection_cas.rs"]
mod selection_cas;

#[path = "coordinator_claims.rs"]
mod coordinator_claims;
