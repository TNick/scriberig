//! Verifies persisted node revisions across durable lifecycle transitions.

use super::*;
use scriberig_core::RootName;
use scriberig_session::{
    PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner, PlanEvidenceReference,
    PlanEvidenceRole, PlanGate, PlanGatePhase, PlanGateRequirement, PlanSemanticReference,
};
use scriberig_store::{RevisionStore, RootStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Commits a plan transition and resolves its exact stored node revision.
///
/// # Parameters
/// - `store`: Durable session repository.
/// - `semantic`: Universal revision and snapshot repository.
/// - `session`: Session containing the current plan projection.
/// - `plan`: Proposed canonical plan after one transition.
/// - `node`: Stable plan-node identity whose revision is checked.
///
/// # Returns
/// The immutable revision ID promoted for the node.
fn commit_and_resolve_node(
    store: &SqliteSessionStore,
    semantic: &SqliteStore,
    session: &mut Session,
    plan: &Plan,
    node: EntityId,
) -> scriberig_core::RevisionId {
    let current = store
        .load(session.id)
        .expect("load current session")
        .expect("session exists");
    let mut proposed = session.clone();
    let plan_slot = proposed
        .plans
        .iter_mut()
        .find(|candidate| candidate.id == plan.id)
        .expect("plan projection");
    let expected_revision = current
        .plans
        .iter()
        .find(|candidate| candidate.id == plan.id)
        .expect("current plan")
        .revision;
    *plan_slot = plan.clone();
    store
        .compare_and_swap_plan(&proposed, plan.id, expected_revision)
        .expect("promote plan transition");
    *session = store
        .load(session.id)
        .expect("reload committed session")
        .expect("session exists");

    let stored_revision = resolve_node_revision(semantic, session.id, plan.id, node);
    let encoded = scriberig_session::PlanSemanticSnapshot::from_plan(plan)
        .expect("encode proposed plan")
        .objects
        .into_iter()
        .find(|object| object.entity == node)
        .expect("encoded node object");
    assert_eq!(stored_revision, encoded.revision);
    stored_revision
}

/// Resolves the current immutable node revision from its semantic root.
///
/// # Parameters
/// - `semantic`: Universal revision and snapshot repository.
/// - `session`: Owning session identity.
/// - `plan`: Stable plan identity.
/// - `node`: Stable node identity to resolve.
///
/// # Returns
/// The stored immutable revision ID for the node.
fn resolve_node_revision(
    semantic: &SqliteStore,
    session: EntityId,
    plan: EntityId,
    node: EntityId,
) -> scriberig_core::RevisionId {
    let root = RootName::new(format!("scriberig-plan-{session}-{plan}")).expect("plan root");
    let snapshot = semantic.read_root(&root).expect("current plan snapshot");
    let revision = semantic
        .resolve(snapshot, node)
        .expect("resolve node in snapshot")
        .expect("node revision mapping");
    let stored = semantic
        .get(revision)
        .expect("read immutable node revision")
        .expect("node revision exists");
    assert_eq!(
        stored.object_type().as_str(),
        scriberig_session::schema::SessionEntityType::PlanNode.as_str()
    );
    revision
}

#[allow(
    clippy::too_many_lines,
    reason = "the test verifies one ordered lifecycle and its revision changes"
)]
/// Resolves exact immutable node revisions for all required lifecycle states.
#[test]
fn stored_node_revision_follows_retry_lifecycle_without_changing_identity() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-plan-lifecycle-revisions-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([151; 16]);
    let branch_id = EntityId::from_bytes([152; 16]);
    let plan_id = EntityId::from_bytes([153; 16]);
    let node_id = EntityId::from_bytes([154; 16]);
    let store = SqliteSessionStore::open(&path).expect("open session store");
    let semantic = SqliteStore::open(&path).expect("open universal store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "persist lifecycle revisions",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(branch_id)),
        title: "validated work".to_owned(),
        purpose: "prove each lifecycle revision".to_owned(),
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
    .expect("node");
    let gate_id = EntityId::from_bytes([155; 16]);
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: node_id,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::ValidationResult,
        },
        waiver: None,
    })
    .expect("entry gate");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan work");
    let mut session = Session {
        id: session_id,
        branches: vec![Branch::new(branch_id, "root").expect("root branch")],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: vec![plan.clone()],
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    store.save(&session).expect("save initial plan");
    let mut revisions = vec![resolve_node_revision(
        &semantic, session_id, plan_id, node_id,
    )];
    let initial_encoded = scriberig_session::PlanSemanticSnapshot::from_plan(&plan)
        .expect("encode initial plan")
        .objects
        .into_iter()
        .find(|object| object.entity == node_id)
        .expect("initial node object");
    assert_eq!(revisions[0], initial_encoded.revision);

    plan.start_with_coordinator_claim(EntityId::from_bytes([156; 16]), "coordinator", None)
        .expect("start coordinator");
    store
        .compare_and_swap_plan(
            &Session {
                plans: vec![plan.clone()],
                ..session.clone()
            },
            plan_id,
            session.plans[0].revision,
        )
        .expect("promote running plan");
    session = store
        .load(session_id)
        .expect("reload running plan")
        .expect("session exists");
    plan = session.plans[0].clone();
    assert_eq!(plan.nodes[0].status, PlanNodeStatus::Planned);

    plan.attach_evidence(PlanEvidenceAssociation {
        id: EntityId::from_bytes([157; 16]),
        owner: PlanEvidenceOwner::Gate(gate_id),
        role: PlanEvidenceRole::Supports,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::ValidationResult,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([158; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::new(),
        },
        note: None,
        actor: None,
    })
    .expect("satisfy entry gate");
    revisions.push(commit_and_resolve_node(
        &store,
        &semantic,
        &mut session,
        &plan,
        node_id,
    ));
    assert_eq!(session.plans[0].nodes[0].status, PlanNodeStatus::Ready);

    let first_branch = EntityId::from_bytes([159; 16]);
    let first_claim = EntityId::from_bytes([160; 16]);
    plan = session.plans[0].clone();
    plan.start_node(node_id, first_branch, first_claim, "worker", &[])
        .expect("start first attempt");
    revisions.push(commit_and_resolve_node(
        &store,
        &semantic,
        &mut session,
        &plan,
        node_id,
    ));
    plan = session.plans[0].clone();
    plan.fail_node(node_id, first_claim)
        .expect("fail first attempt");
    revisions.push(commit_and_resolve_node(
        &store,
        &semantic,
        &mut session,
        &plan,
        node_id,
    ));
    plan = session.plans[0].clone();
    plan.retry_failed_node(node_id, &[])
        .expect("retry failed attempt");
    revisions.push(commit_and_resolve_node(
        &store,
        &semantic,
        &mut session,
        &plan,
        node_id,
    ));

    let second_branch = EntityId::from_bytes([161; 16]);
    let second_claim = EntityId::from_bytes([162; 16]);
    plan = session.plans[0].clone();
    plan.start_node(node_id, second_branch, second_claim, "worker", &[])
        .expect("start retry");
    revisions.push(commit_and_resolve_node(
        &store,
        &semantic,
        &mut session,
        &plan,
        node_id,
    ));
    plan = session.plans[0].clone();
    let branch_result = scriberig_session::BranchResult {
        id: EntityId::from_bytes([163; 16]),
        branch: second_branch,
        status: scriberig_session::BranchStatus::Completed,
        summary: None,
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    plan.complete_node(node_id, second_claim, &branch_result)
        .expect("complete retry");
    let mut worker_branch = Branch::new(second_branch, "second worker").expect("worker branch");
    worker_branch.status = BranchStatus::Completed;
    worker_branch.result = Some(branch_result);
    session.branches.push(worker_branch);
    revisions.push(commit_and_resolve_node(
        &store,
        &semantic,
        &mut session,
        &plan,
        node_id,
    ));

    assert!(revisions.windows(2).all(|pair| pair[0] != pair[1]));
    assert_eq!(session.plans[0].nodes[0].id, node_id);
    assert_eq!(session.plans[0].nodes[0].status, PlanNodeStatus::Completed);
    drop(semantic);
    drop(store);
    let _ = std::fs::remove_file(path);
}
