//! Proves one worker claim CAS rewrites only the claimed node and plan rows.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use scriberig_core::{EntityId, RootName, SnapshotId};
use scriberig_session::{
    Branch, Plan, PlanEvidenceKind, PlanEvidenceOwner, PlanEvidenceRole, PlanGate, PlanGatePhase,
    PlanGateRequirement, PlanNodeStatus, PlanNodeSummary, PlanStatus, Session, SessionStore,
    schema::SessionEntityType,
};
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store::RootStore;
use scriberig_store_sqlite::SqliteStore;

use super::fixtures::{association, planned_node};
use super::rows;

/// Durable rows describing one exact plan-root snapshot.
struct DurableState {
    /// Plan-root snapshot the rows were read from.
    snapshot: SnapshotId,
    /// Every stored revision row keyed by immutable revision ID.
    revisions: BTreeMap<Vec<u8>, rows::RevisionRow>,
    /// Snapshot entity-to-revision mapping.
    entries: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Derived rollup rows keyed by stored node identity text.
    rollups: BTreeMap<String, rows::RollupRow>,
}

/// Reads the durable rows for the plan root's current snapshot.
///
/// # Parameters
/// - `path`: Database shared by the session and universal stores.
/// - `semantic`: Universal revision and snapshot repository.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable semantic plan identity.
///
/// # Returns
/// The current plan-root snapshot and its stored rows.
fn durable_state(
    path: &Path,
    semantic: &SqliteStore,
    session: EntityId,
    plan: EntityId,
) -> DurableState {
    let root = RootName::new(format!("scriberig-plan-{session}-{plan}")).expect("plan root");
    let snapshot = semantic.read_root(&root).expect("current plan snapshot");
    DurableState {
        snapshot,
        revisions: rows::revision_rows(path),
        entries: rows::snapshot_entries(path, &snapshot),
        rollups: rows::rollup_rows(path, session, plan),
    }
}

/// Asserts one claim CAS appended only the claimed node and plan rows.
///
/// # Parameters
/// - `before`: Durable rows before the claim promotion.
/// - `after`: Durable rows after the claim promotion.
/// - `plan`: Stable plan identity whose record must advance.
/// - `claimed`: Node whose claim is the only node-level write.
fn assert_claim_write_scope(
    before: &DurableState,
    after: &DurableState,
    plan: EntityId,
    claimed: EntityId,
) {
    // No stored revision row is rewritten or removed by the claim.
    for (revision, row) in &before.revisions {
        assert_eq!(
            after.revisions.get(revision),
            Some(row),
            "an existing immutable revision row must survive a claim promotion"
        );
    }

    // Exactly three new revision rows appear: the claimed node, the plan
    // record whose mutation revision advanced, and the new mutation revision.
    let mut appended_types = after
        .revisions
        .iter()
        .filter(|(revision, _)| !before.revisions.contains_key(*revision))
        .map(|(_, row)| row.object_type.clone())
        .collect::<Vec<_>>();
    appended_types.sort();
    assert_eq!(
        appended_types,
        [
            SessionEntityType::Plan.as_str().to_owned(),
            SessionEntityType::PlanNode.as_str().to_owned(),
            SessionEntityType::PlanRevision.as_str().to_owned(),
        ],
        "a claim appends only the claimed node, plan, and plan-revision rows"
    );

    // The snapshot mapping changes for exactly the plan and claimed node.
    let changed = before
        .entries
        .iter()
        .filter(|(entity, revision)| after.entries.get(*entity) != Some(revision))
        .map(|(entity, _)| entity.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        changed,
        [plan.as_bytes().to_vec(), claimed.as_bytes().to_vec()]
            .into_iter()
            .collect(),
        "only the plan and claimed node change their stored revision"
    );
    assert!(
        before
            .entries
            .keys()
            .all(|entity| after.entries.contains_key(entity)),
        "a claim removes no semantic object"
    );
    assert_eq!(
        after.entries.len(),
        before.entries.len() + 1,
        "a claim adds exactly the new plan-revision record"
    );
    for (entity, revision) in &after.entries {
        if let Some(previous) = before.entries.get(entity)
            && previous == revision
        {
            assert!(
                before.revisions.contains_key(revision),
                "an unchanged entity must reuse its pre-existing revision row"
            );
        }
    }

    // Derived rollup rows advance their snapshot provenance but keep the
    // stored semantics of every node the claim did not touch.
    assert_eq!(
        after.rollups.len(),
        before.rollups.len(),
        "a claim adds or removes no derived rollup row"
    );
    for (node, before_row) in &before.rollups {
        let after_row = after.rollups.get(node).expect("rollup row remains present");
        assert_eq!(
            after_row.source_snapshot,
            after.snapshot.to_string(),
            "every derived rollup row advances to the promoted snapshot"
        );
        if node == &claimed.to_string() {
            continue;
        }
        assert_eq!(
            after_row.node_revision, before_row.node_revision,
            "an untouched node keeps its canonical revision in the rollup row"
        );
        assert_eq!(
            after_row.parent_node_id, before_row.parent_node_id,
            "an untouched node keeps its structural parent"
        );
        assert_eq!(
            after_row.direct_status, before_row.direct_status,
            "an untouched node keeps its direct status"
        );
        assert_eq!(
            after_row.effective_status, before_row.effective_status,
            "an untouched node keeps its effective status"
        );
        assert_eq!(
            after_row.payload, before_row.payload,
            "an untouched node's derived rollup payload is not rewritten"
        );
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one claim's durable write scope is asserted against the rows stored around it"
)]
/// Claims two disjoint nodes and proves each write touches only its own node.
#[test]
fn claim_cas_rewrites_only_the_claimed_node_claim_and_plan_rows() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-plan-scope-claim-{}.sqlite",
        EntityId::new()
    ));
    let store = SqliteSessionStore::open(&path).expect("open session store");
    let semantic = SqliteStore::open(&path).expect("open universal store");
    let session = EntityId::from_bytes([180; 16]);
    let branch = EntityId::from_bytes([181; 16]);
    let plan_id = EntityId::from_bytes([182; 16]);
    let claimed = EntityId::from_bytes([183; 16]);
    let untouched = EntityId::from_bytes([184; 16]);
    let second = EntityId::from_bytes([185; 16]);
    let entry_gate = EntityId::from_bytes([187; 16]);
    let gate_evidence = EntityId::from_bytes([188; 16]);
    let node_gate = EntityId::from_bytes([189; 16]);
    let node_evidence = EntityId::from_bytes([190; 16]);
    let summary = EntityId::from_bytes([191; 16]);
    let coordinator_claim = EntityId::from_bytes([192; 16]);
    let first_claim = EntityId::from_bytes([193; 16]);
    let first_worker_branch = EntityId::from_bytes([194; 16]);
    let second_claim = EntityId::from_bytes([195; 16]);
    let second_worker_branch = EntityId::from_bytes([196; 16]);

    let mut plan = Plan::new(
        plan_id,
        session,
        scriberig_session::ContinuationAnchor::BranchHead { branch },
        "measure one claim mutation's durable write scope",
    )
    .expect("plan");
    plan.add_node(planned_node(claimed, branch, "first claimed work"))
        .expect("first claimed node");
    plan.add_node(planned_node(untouched, branch, "unrelated work"))
        .expect("unrelated node");
    plan.add_node(planned_node(second, branch, "second claimed work"))
        .expect("second claimed node");
    plan.add_dependency(claimed, untouched)
        .expect("dependency edge");
    plan.add_gate(PlanGate {
        id: entry_gate,
        owner: claimed,
        phase: PlanGatePhase::Entry,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::Artifact,
        },
        waiver: None,
    })
    .expect("entry gate");
    plan.add_gate(PlanGate {
        id: node_gate,
        owner: untouched,
        phase: PlanGatePhase::Completion,
        requirement: PlanGateRequirement::EvidencePresent {
            kind: PlanEvidenceKind::ValidationResult,
        },
        waiver: None,
    })
    .expect("completion gate");
    plan.attach_evidence(association(
        gate_evidence,
        PlanEvidenceOwner::Gate(entry_gate),
        PlanEvidenceRole::Supports,
    ))
    .expect("satisfy entry gate");
    plan.attach_evidence(association(
        node_evidence,
        PlanEvidenceOwner::Node(untouched),
        PlanEvidenceRole::Informational,
    ))
    .expect("unrelated node evidence");
    plan.update_summary(PlanNodeSummary {
        id: summary,
        node: untouched,
        text: "unrelated node progress".to_owned(),
        sources: Vec::new(),
    })
    .expect("unrelated node summary");
    plan.transition(PlanStatus::Planned).expect("plan work");

    store
        .save(&Session {
            id: session,
            branches: vec![Branch::new(branch, "root").expect("root branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save plan fixture");

    let stored = store.load(session).expect("load fixture").expect("fixture");
    let mut started = stored.clone();
    started.plans[0]
        .start_with_coordinator_claim(coordinator_claim, "coordinator-a", None)
        .expect("coordinator claim");
    store
        .compare_and_swap_plan(&started, plan_id, stored.plans[0].revision)
        .expect("promote running plan");
    let running = store
        .load(session)
        .expect("reload")
        .expect("running session");
    store
        .rebuild_plan_rollup_index(session, plan_id)
        .expect("seed derived rollups");
    let before_first = durable_state(&path, &semantic, session, plan_id);
    assert!(
        rows::entities_holding_identity(&path, &before_first.snapshot, first_claim).is_empty(),
        "no stored object records the worker claim before the mutation"
    );

    let mut proposed = running.clone();
    proposed.plans[0]
        .start_node(claimed, first_worker_branch, first_claim, "worker-a", &[])
        .expect("claim first node");
    store
        .compare_and_swap_plan(&proposed, plan_id, running.plans[0].revision)
        .expect("promote first claim");
    let after_first = durable_state(&path, &semantic, session, plan_id);
    assert_claim_write_scope(&before_first, &after_first, plan_id, claimed);

    // The worker claim lives only in the claimed node's row; the coordinator
    // claim stays in the plan record.
    assert_eq!(
        rows::entities_holding_identity(&path, &after_first.snapshot, first_claim),
        vec![claimed.as_bytes().to_vec()],
        "exactly the claimed node's row records the worker claim"
    );
    assert_eq!(
        rows::entities_holding_identity(&path, &after_first.snapshot, coordinator_claim),
        vec![plan_id.as_bytes().to_vec()],
        "the coordinator claim stays in the plan record"
    );
    let claimed_plan = store
        .load(session)
        .expect("reload")
        .expect("claimed session");
    let node_state = |id: EntityId| {
        claimed_plan.plans[0]
            .nodes
            .iter()
            .find(|node| node.id == id)
            .expect("node")
            .clone()
    };
    assert_eq!(
        node_state(claimed).claim.as_ref().expect("first claim").id,
        first_claim
    );
    assert_eq!(node_state(claimed).status, PlanNodeStatus::Running);
    assert_eq!(node_state(untouched).claim, None);
    assert_eq!(node_state(second).claim, None);
    assert_eq!(node_state(untouched).status, PlanNodeStatus::Planned);

    let mut proposed = claimed_plan.clone();
    proposed.plans[0]
        .start_node(second, second_worker_branch, second_claim, "worker-b", &[])
        .expect("claim second node");
    store
        .compare_and_swap_plan(&proposed, plan_id, claimed_plan.plans[0].revision)
        .expect("promote second claim");
    let after_second = durable_state(&path, &semantic, session, plan_id);
    assert_claim_write_scope(&after_first, &after_second, plan_id, second);

    // The first claim's node row survives the disjoint second claim untouched,
    // and each worker claim stays inside its own node row.
    assert_eq!(
        after_second.entries.get(claimed.as_bytes().as_slice()),
        after_first.entries.get(claimed.as_bytes().as_slice()),
        "the previously claimed node keeps its exact stored revision"
    );
    assert_eq!(
        rows::entities_holding_identity(&path, &after_second.snapshot, first_claim),
        vec![claimed.as_bytes().to_vec()],
        "the first worker claim stays in the first node's row"
    );
    assert_eq!(
        rows::entities_holding_identity(&path, &after_second.snapshot, second_claim),
        vec![second.as_bytes().to_vec()],
        "exactly the second claimed node's row records the second worker claim"
    );
    assert_eq!(
        after_second.rollups[&claimed.to_string()].direct_status,
        format!("{:?}", PlanNodeStatus::Running),
        "the first claim's derived rollup row stays advanced"
    );
    assert_eq!(
        after_second.rollups[&second.to_string()].direct_status,
        format!("{:?}", PlanNodeStatus::Running),
        "the second claim advances only its own rollup row"
    );
    assert_eq!(
        after_second.rollups[&untouched.to_string()].direct_status,
        format!("{:?}", PlanNodeStatus::Planned),
        "the unrelated node's derived rollup row stays Planned"
    );

    drop(semantic);
    drop(store);
    let _ = std::fs::remove_file(path);
}
