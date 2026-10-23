//! Verifies one worker claim mutation touches only the claimed node and plan revision.

use super::*;
use std::collections::BTreeMap;

use scriberig_core::{RevisionId, RootName, SnapshotId};
use scriberig_session::{
    PlanEvidenceAssociation, PlanEvidenceKind, PlanEvidenceOwner, PlanEvidenceReference,
    PlanEvidenceRole, PlanGate, PlanGatePhase, PlanGateRequirement, PlanNodeParent,
    PlanSemanticReference, schema::SessionEntityType,
};
use scriberig_store::{RevisionStore, RootStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Builds one planned semantic node with no assignment or execution attempt.
///
/// # Parameters
/// - `id`: Stable node identity.
/// - `branch`: Root branch that structurally parents the node.
/// - `title`: Human-readable node title.
///
/// # Returns
/// A planned node parented by the plan's root branch.
fn planned_node(id: EntityId, branch: EntityId, title: &str) -> PlanNode {
    PlanNode {
        id,
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

/// Reads the current plan root snapshot and every stored object mapping in it.
///
/// # Parameters
/// - `semantic`: Universal revision and snapshot repository.
/// - `session`: Owning session identity.
/// - `plan`: Stable plan identity whose root is read.
///
/// # Returns
/// The exact root snapshot and its entity to revision-ID mapping.
fn plan_object_mappings(
    semantic: &SqliteStore,
    session: EntityId,
    plan: EntityId,
) -> (SnapshotId, BTreeMap<EntityId, RevisionId>) {
    let root = RootName::new(format!("scriberig-plan-{session}-{plan}")).expect("plan root");
    let snapshot = semantic.read_root(&root).expect("current plan snapshot");
    let mappings = semantic
        .iter(snapshot)
        .expect("read plan objects")
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    (snapshot, mappings)
}

/// Resolves the stored semantic object type for one revision mapping.
///
/// # Parameters
/// - `semantic`: Universal revision and snapshot repository.
/// - `revision`: Immutable revision ID to read.
///
/// # Returns
/// The stored object type identifier.
fn stored_object_type(semantic: &SqliteStore, revision: RevisionId) -> String {
    semantic
        .get(revision)
        .expect("read stored revision")
        .expect("stored revision exists")
        .object_type()
        .as_str()
        .to_owned()
}

#[allow(
    clippy::too_many_lines,
    reason = "the test compares the complete semantic object mapping before and after one claim"
)]
/// Claims one node and proves only that node and the plan mutation revision change.
#[test]
fn claim_mutation_revises_only_the_claimed_node_and_plan_mutation_revision() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-claim-revision-scope-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([240; 16]);
    let branch_id = EntityId::from_bytes([241; 16]);
    let plan_id = EntityId::from_bytes([242; 16]);
    let claimed_node = EntityId::from_bytes([243; 16]);
    let unrelated_node = EntityId::from_bytes([244; 16]);
    let gate_id = EntityId::from_bytes([245; 16]);
    let evidence_id = EntityId::from_bytes([246; 16]);
    let store = SqliteSessionStore::open(&path).expect("open session store");
    let semantic = SqliteStore::open(&path).expect("open universal store");

    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "prove one claim mutation's semantic scope",
    )
    .expect("plan");
    plan.add_node(planned_node(claimed_node, branch_id, "claimed work"))
        .expect("claimed node");
    plan.add_node(planned_node(unrelated_node, branch_id, "unrelated work"))
        .expect("unrelated node");
    plan.add_gate(PlanGate {
        id: gate_id,
        owner: claimed_node,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::Artifact,
        },
        waiver: None,
    })
    .expect("entry gate");
    plan.attach_evidence(PlanEvidenceAssociation {
        id: evidence_id,
        owner: PlanEvidenceOwner::Gate(gate_id),
        role: PlanEvidenceRole::Supports,
        evidence: PlanEvidenceReference {
            kind: PlanEvidenceKind::Artifact,
            reference: PlanSemanticReference {
                entity: EntityId::from_bytes([247; 16]),
                revision: None,
                snapshot: None,
            },
            attributes: std::collections::BTreeMap::new(),
        },
        note: None,
        actor: None,
    })
    .expect("satisfy entry gate");
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan work");
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(branch_id, "root").expect("root branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save initial plan");

    let mut session = store
        .load(session_id)
        .expect("load stored session")
        .expect("stored session");
    let mut running = session.clone();
    running.plans[0]
        .start_with_coordinator_claim(EntityId::from_bytes([248; 16]), "coordinator", None)
        .expect("start coordinator");
    store
        .compare_and_swap_plan(&running, plan_id, session.plans[0].revision)
        .expect("promote running plan");
    session = store
        .load(session_id)
        .expect("reload running session")
        .expect("running session");

    let claim_revision = session.plans[0].revision;
    let unrelated_before = session
        .plans
        .iter()
        .flat_map(|candidate| candidate.nodes.iter())
        .find(|node| node.id == unrelated_node)
        .expect("unrelated node before claim")
        .clone();
    assert_eq!(
        unrelated_before.claim, None,
        "no node holds a claim before the measured mutation"
    );
    let (before_snapshot, before) = plan_object_mappings(&semantic, session_id, plan_id);
    let before_rollup_snapshot = store
        .rebuild_plan_rollup_index(session_id, plan_id)
        .expect("build rollups before claim");
    let before_rollups = store
        .read_plan_rollup_index(session_id, plan_id, before_rollup_snapshot)
        .expect("read rollups before claim")
        .expect("current rollups before claim");

    let mut proposed = session.clone();
    proposed.plans[0]
        .start_node(
            claimed_node,
            EntityId::from_bytes([249; 16]),
            EntityId::from_bytes([250; 16]),
            "worker-a",
            &[],
        )
        .expect("claim one ready node");
    store
        .compare_and_swap_plan(&proposed, plan_id, claim_revision)
        .expect("promote claim mutation");

    let reloaded = store
        .load(session_id)
        .expect("reload claimed session")
        .expect("claimed session");
    let claimed_plan = &reloaded.plans[0];
    let (after_snapshot, after) = plan_object_mappings(&semantic, session_id, plan_id);

    // (d) the plan mutation revision advanced exactly once.
    assert_eq!(claimed_plan.revision, claim_revision + 1);
    assert_ne!(after_snapshot, before_snapshot);

    // (a) the claimed node's stored semantic revision changed.
    assert_ne!(
        before[&claimed_node], after[&claimed_node],
        "the claimed node must receive a new semantic revision"
    );
    assert_eq!(
        claimed_plan
            .nodes
            .iter()
            .find(|node| node.id == claimed_node)
            .expect("claimed node")
            .status,
        PlanNodeStatus::Running,
        "the claimed node is running"
    );

    // (b) every other node keeps its exact stored semantic revision ID.
    assert_eq!(
        before[&unrelated_node], after[&unrelated_node],
        "an unrelated PlanNode must keep its exact semantic revision ID"
    );
    assert_eq!(
        claimed_plan
            .nodes
            .iter()
            .find(|node| node.id == unrelated_node)
            .expect("unrelated node"),
        &unrelated_before,
        "the unrelated node's durable fields are untouched"
    );

    // (c) gate, evidence, and structural objects keep their exact revisions.
    assert_eq!(
        before[&gate_id], after[&gate_id],
        "an untouched PlanGate must keep its exact semantic revision ID"
    );
    assert_eq!(
        before[&evidence_id], after[&evidence_id],
        "an untouched PlanEvidence association must keep its exact semantic revision ID"
    );

    // The only changed mappings are the claimed node and the plan mutation record.
    let changed = before
        .iter()
        .filter(|(entity, revision)| after.get(*entity) != Some(revision))
        .map(|(entity, _)| *entity)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        changed,
        [plan_id, claimed_node].into_iter().collect(),
        "only the plan record and the claimed node change revision"
    );
    let added = after
        .keys()
        .filter(|entity| !before.contains_key(*entity))
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(added.len(), 1, "the claim adds one semantic object");
    assert_eq!(
        stored_object_type(&semantic, after[&added[0]]),
        SessionEntityType::PlanRevision.as_str(),
        "the added object is the new plan mutation revision record"
    );
    assert!(
        before.keys().all(|entity| after.contains_key(entity)),
        "no semantic object is removed by the claim"
    );

    // The derived rollup cache advances only the claimed node's row.
    let after_rollup_snapshot = claimed_plan.source_snapshot.expect("claimed plan snapshot");
    let after_rollups = store
        .read_plan_rollup_index(session_id, plan_id, after_rollup_snapshot)
        .expect("read rollups after claim")
        .expect("incrementally advanced rollups");
    assert_eq!(
        before_rollups[&unrelated_node], after_rollups[&unrelated_node],
        "the unrelated node's derived rollup row is unchanged"
    );
    assert_eq!(
        after_rollups[&claimed_node].direct_status,
        PlanNodeStatus::Running,
        "the claimed node's derived rollup row advanced"
    );

    drop(semantic);
    drop(store);
    let _ = std::fs::remove_file(path);
}
