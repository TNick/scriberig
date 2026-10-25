//! Proves compact plan reads hydrate only the objects the shared views need.

use std::path::Path;

use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{
    Branch, Plan, PlanEvidenceKind, PlanEvidenceOwner, PlanEvidenceRole, PlanGate, PlanGatePhase,
    PlanGateRequirement, PlanNodeStatus, PlanNodeSummary, PlanStatus, Session, SessionStore,
    schema::SessionEntityType,
};
use scriberig_session_sqlite::SqliteSessionStore;

use super::fixtures::{association, planned_node};
use super::rows;

/// Reads one stored object-type census for an exact plan snapshot.
///
/// # Parameters
/// - `path`: Database shared by the session and universal stores.
/// - `snapshot`: Exact plan-root snapshot to census.
/// - `object_type`: Semantic object type to count.
///
/// # Returns
/// Number of stored objects of that type in the snapshot.
fn stored_count(path: &Path, snapshot: &SnapshotId, object_type: SessionEntityType) -> i64 {
    rows::snapshot_object_counts(path, snapshot)
        .get(object_type.as_str())
        .copied()
        .unwrap_or(0)
}

#[allow(
    clippy::too_many_lines,
    reason = "the compact read scope is asserted against the durable rows it must skip"
)]
/// Reads one populated plan through the detail and compact paths and compares
/// the semantic objects each path hydrates.
#[test]
fn compact_plan_read_hydrates_only_view_required_objects() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-plan-scope-compact-{}.sqlite",
        EntityId::new()
    ));
    let store = SqliteSessionStore::open(&path).expect("open session store");
    let session = EntityId::from_bytes([200; 16]);
    let branch = EntityId::from_bytes([201; 16]);
    let plan_id = EntityId::from_bytes([202; 16]);
    let claimed = EntityId::from_bytes([203; 16]);
    let unrelated = EntityId::from_bytes([204; 16]);
    let second = EntityId::from_bytes([205; 16]);
    let entry_gate = EntityId::from_bytes([206; 16]);
    let node_gate = EntityId::from_bytes([207; 16]);
    let gate_evidence = EntityId::from_bytes([208; 16]);
    let node_evidence = EntityId::from_bytes([209; 16]);
    let summary = EntityId::from_bytes([210; 16]);
    let mut plan = Plan::new(
        plan_id,
        session,
        scriberig_session::ContinuationAnchor::BranchHead { branch },
        "measure compact plan read scope",
    )
    .expect("plan");
    plan.add_node(planned_node(claimed, branch, "claimable work"))
        .expect("claimable node");
    plan.add_node(planned_node(unrelated, branch, "unrelated work"))
        .expect("unrelated node");
    plan.add_node(planned_node(second, branch, "second work"))
        .expect("second node");
    plan.add_dependency(claimed, unrelated)
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
        owner: unrelated,
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
        PlanEvidenceOwner::Node(unrelated),
        PlanEvidenceRole::Informational,
    ))
    .expect("unrelated node evidence");
    plan.update_summary(PlanNodeSummary {
        id: summary,
        node: unrelated,
        text: "the compact projection must keep this preview".to_owned(),
        sources: Vec::new(),
    })
    .expect("node summary");
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

    let stored = store.load(session).expect("load").expect("fixture");
    let mut running = stored.clone();
    running.plans[0]
        .start_with_coordinator_claim(EntityId::from_bytes([211; 16]), "coordinator-a", None)
        .expect("coordinator claim");
    store
        .compare_and_swap_plan(&running, plan_id, stored.plans[0].revision)
        .expect("promote running plan");
    let running = store.load(session).expect("reload").expect("running");
    let historical = running.plans[0]
        .source_snapshot
        .expect("running plan snapshot");

    let mut proposed = running.clone();
    proposed.plans[0]
        .start_node(
            claimed,
            EntityId::from_bytes([212; 16]),
            EntityId::from_bytes([213; 16]),
            "worker-a",
            &[],
        )
        .expect("claim one node");
    store
        .compare_and_swap_plan(&proposed, plan_id, running.plans[0].revision)
        .expect("promote claim");
    let promoted = store.load(session).expect("reload").expect("claimed");
    let snapshot = promoted.plans[0]
        .source_snapshot
        .expect("claimed plan snapshot");
    assert_ne!(snapshot, historical, "the claim promotes a new snapshot");

    // Every detail object the compact read must skip is durably present.
    assert_eq!(
        stored_count(&path, &snapshot, SessionEntityType::Plan),
        1_i64
    );
    assert_eq!(
        stored_count(&path, &snapshot, SessionEntityType::PlanNode),
        3_i64,
        "three PlanNode rows are stored"
    );
    assert_eq!(
        stored_count(&path, &snapshot, SessionEntityType::PlanGate),
        2_i64,
        "two PlanGate rows are stored"
    );
    assert_eq!(
        stored_count(&path, &snapshot, SessionEntityType::PlanEvidence),
        2_i64,
        "two PlanEvidence rows are stored"
    );
    assert_eq!(
        stored_count(&path, &snapshot, SessionEntityType::PlanDependency),
        1_i64,
        "the dependency edge row is stored"
    );
    assert_eq!(
        stored_count(&path, &snapshot, SessionEntityType::PlanSummary),
        1_i64,
        "the current summary row is stored"
    );
    assert_eq!(
        stored_count(&path, &snapshot, SessionEntityType::PlanSummaryLink),
        1_i64,
        "the current summary link row is stored"
    );
    assert!(
        stored_count(&path, &snapshot, SessionEntityType::PlanRevision) >= 4,
        "plan revision history rows are stored for every promoted mutation"
    );

    let detail = SessionStore::load_plan_at_snapshot(&store, session, plan_id, snapshot)
        .expect("detail read")
        .expect("detail plan");
    assert_eq!(detail.gates.len(), 2, "the detail read hydrates both gates");
    assert_eq!(
        detail.evidence.len(),
        2,
        "the detail read hydrates both evidence associations"
    );
    assert!(
        !detail.history.is_empty(),
        "the detail read hydrates plan revision history"
    );

    let compact = SessionStore::load_plan_projection(&store, session, plan_id, None)
        .expect("compact read")
        .expect("compact plan");
    assert_eq!(compact.source_snapshot, Some(snapshot));
    assert!(
        compact.gates.is_empty() && compact.evidence.is_empty(),
        "the compact read hydrates no gate or evidence object although both are stored"
    );
    assert!(
        compact.history.is_empty(),
        "the compact read hydrates no plan revision history although revision rows are stored"
    );
    assert_eq!(compact.nodes.len(), 3, "every node is still enumerable");
    assert_eq!(
        compact.dependencies.len(),
        1,
        "dependency blockers remain enumerable for the compact node view"
    );
    assert_eq!(
        compact.summaries.len(),
        1,
        "the current summary is hydrated because the shared node view previews it"
    );
    assert_eq!(
        compact
            .nodes
            .iter()
            .find(|node| node.id == claimed)
            .expect("claimed node")
            .status,
        PlanNodeStatus::Running,
        "the compact read reflects the promoted claim"
    );

    // The gate and evidence counts a compact view exposes come from the
    // snapshot-bound derived cache, which matches reference recomputation over
    // hydrated detail objects.
    let cached = SessionStore::load_plan_rollups(&store, session, plan_id, snapshot)
        .expect("cached rollups")
        .expect("cache matches the current snapshot");
    assert_eq!(
        cached,
        detail.node_rollups().expect("reference rollups"),
        "cached gate and evidence counts equal reference recomputation from detail"
    );
    assert_eq!(
        cached[&claimed].entry_gates.satisfied, 1,
        "the satisfied entry-gate count is read from the derived cache"
    );
    assert_eq!(
        cached[&unrelated].completion_gates.pending, 1,
        "the pending completion-gate count is read from the derived cache"
    );
    assert_eq!(
        cached[&claimed].evidence_count, 1,
        "the evidence count is read from the derived cache"
    );

    // A pinned historical compact read still hydrates detail: no snapshot-bound
    // rollup cache exists for an older snapshot, so the shared evaluator needs
    // the gate and evidence objects to produce the same counts.
    let historical_compact =
        SessionStore::load_plan_projection(&store, session, plan_id, Some(historical))
            .expect("historical compact read")
            .expect("historical plan");
    assert_eq!(
        historical_compact.source_snapshot,
        Some(historical),
        "the pinned read resolves exactly the requested snapshot"
    );
    assert_eq!(
        historical_compact.gates.len(),
        2,
        "a pinned historical compact read hydrates gates because the evaluator needs them"
    );

    drop(store);
    let _ = std::fs::remove_file(path);
}
