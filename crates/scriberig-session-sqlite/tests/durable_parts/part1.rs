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

/// Verifies selection compare-and-swap preserves concurrent branch updates.
#[test]
fn execution_selection_compare_and_swap_is_atomic() {
    let session_id = EntityId::from_bytes([11; 16]);
    let root_id = EntityId::from_bytes([12; 16]);
    let child_id = EntityId::from_bytes([13; 16]);
    let store = Arc::new(SqliteSessionStore::open_in_memory().expect("open store"));
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(root_id, "root").expect("root")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial session");
    let mut proposed = store
        .load(session_id)
        .expect("load selection snapshot")
        .expect("session exists");
    proposed
        .execution
        .replace(
            0,
            Some("provider-a".to_owned()),
            Some("model-a".to_owned()),
            Some("careful".to_owned()),
        )
        .expect("propose selection");
    proposed
        .execution
        .history
        .push(scriberig_session::SessionExecutionChange {
            revision: proposed.execution.revision,
            kind: scriberig_session::AgentActivityKind::ModelSelectionChanged,
            summary: "Model selected: model-a".to_owned(),
            provider_id: proposed.execution.provider_id.clone(),
            model_id: proposed.execution.model_id.clone(),
            thinking_level: proposed.execution.thinking_level.clone(),
        });
    let mut branch_update = store
        .load(session_id)
        .expect("load branch snapshot")
        .expect("session exists");
    branch_update
        .branches
        .push(Branch::new(child_id, "new branch").expect("child"));
    store.save(&branch_update).expect("save concurrent branch");
    branch_update
        .execution
        .replace(
            0,
            Some("provider-b".to_owned()),
            Some("model-b".to_owned()),
            Some("steady".to_owned()),
        )
        .expect("propose competing selection");

    let first_store = Arc::clone(&store);
    let first =
        std::thread::spawn(move || first_store.compare_and_swap_execution_selection(&proposed, 0));
    let second_store = Arc::clone(&store);
    let second = std::thread::spawn(move || {
        second_store.compare_and_swap_execution_selection(&branch_update, 0)
    });
    let outcomes = [
        first.join().expect("first mutation"),
        second.join().expect("second mutation"),
    ];
    assert_eq!(outcomes.iter().filter(|value| value.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|value| matches!(value, Err(SelectionStoreError::RevisionConflict { .. })))
            .count(),
        1
    );
    let stored = store
        .load(session_id)
        .expect("reload session")
        .expect("session exists");
    assert_eq!(stored.branches.len(), 2);
    assert!(matches!(
        stored.execution.model_id.as_deref(),
        Some("model-a" | "model-b")
    ));
    assert_eq!(stored.execution.history.len(), 1);
    assert_eq!(
        stored.execution.history[0].provider_id.as_deref(),
        stored.execution.provider_id.as_deref()
    );
}

#[path = "coordinator_claims.rs"]
mod coordinator_claims;
