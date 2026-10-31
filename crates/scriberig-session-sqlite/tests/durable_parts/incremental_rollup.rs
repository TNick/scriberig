//! Verifies persisted plan rollups advance through incremental mutations.

use super::*;
use scriberig_session::{PlanNodeParent, PlanNodeRollupIndex};

/// Builds one planned semantic node at the requested structural location.
///
/// # Parameters
/// - `id`: Stable node identity.
/// - `parent`: Branch or node parent relation.
/// - `title`: Human-readable node title.
///
/// # Returns
/// A planned node with no assignment or execution attempt.
fn planned_node(id: EntityId, parent: PlanNodeParent, title: &str) -> PlanNode {
    PlanNode {
        id,
        parent: Some(parent),
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

/// Updates one leaf and verifies snapshot-bound persisted ancestor rollups.
#[test]
fn plan_cas_persists_incremental_ancestor_rollups() {
    let session_id = EntityId::from_bytes([190; 16]);
    let branch_id = EntityId::from_bytes([191; 16]);
    let plan_id = EntityId::from_bytes([192; 16]);
    let parent_id = EntityId::from_bytes([193; 16]);
    let leaf_id = EntityId::from_bytes([194; 16]);
    let unrelated_id = EntityId::from_bytes([195; 16]);
    let store = SqliteSessionStore::open_in_memory().expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: branch_id },
        "incremental rollup",
    )
    .expect("plan");
    plan.add_node(planned_node(
        parent_id,
        PlanNodeParent::Branch(branch_id),
        "parent",
    ))
    .expect("parent node");
    plan.nodes
        .iter_mut()
        .find(|node| node.id == parent_id)
        .expect("parent node")
        .status = PlanNodeStatus::Completed;
    plan.add_node(planned_node(
        leaf_id,
        PlanNodeParent::Node(parent_id),
        "leaf",
    ))
    .expect("leaf node");
    plan.add_node(planned_node(
        unrelated_id,
        PlanNodeParent::Branch(branch_id),
        "unrelated root",
    ))
    .expect("unrelated node");
    plan.add_gate(scriberig_session::PlanGate {
        id: EntityId::from_bytes([196; 16]),
        owner: parent_id,
        phase: scriberig_session::PlanGatePhase::Completion,
        requirement: scriberig_session::PlanGateRequirement::RequiredChildCompleted {
            child: leaf_id,
        },
        waiver: None,
    })
    .expect("required child gate");
    store
        .save(&Session {
            id: session_id,
            branches: vec![Branch::new(branch_id, "root").expect("branch")],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: vec![plan],
            execution: scriberig_session::SessionExecutionSelection::default(),
        })
        .expect("save semantic plan");
    let first_snapshot = store
        .rebuild_plan_rollup_index(session_id, plan_id)
        .expect("build base rollups");
    let before = store
        .read_plan_rollup_index(session_id, plan_id, first_snapshot)
        .expect("read base cache")
        .expect("base cache is current");
    let expected_revision = store
        .load(session_id)
        .expect("load plan")
        .expect("stored session")
        .plans[0]
        .revision;
    let mut proposed = store
        .load(session_id)
        .expect("load plan for mutation")
        .expect("stored session");
    let node = proposed.plans[0]
        .nodes
        .iter_mut()
        .find(|node| node.id == leaf_id)
        .expect("leaf node");
    node.status = PlanNodeStatus::Failed;
    proposed.plans[0].revision = expected_revision + 1;
    store
        .compare_and_swap_plan(&proposed, plan_id, expected_revision)
        .expect("commit leaf status");

    let current = store
        .load(session_id)
        .expect("reload semantic plan")
        .expect("stored session");
    let snapshot = current.plans[0].source_snapshot.expect("current snapshot");
    let after = store
        .read_plan_rollup_index(session_id, plan_id, snapshot)
        .expect("read updated cache")
        .expect("incrementally updated cache remains current");
    assert_ne!(snapshot, first_snapshot);
    assert_eq!(after[&leaf_id].direct_status, PlanNodeStatus::Failed);
    assert_eq!(after[&parent_id].effective_status, PlanNodeStatus::Failed);
    assert_eq!(after[&unrelated_id], before[&unrelated_id]);
    let reference =
        PlanNodeRollupIndex::rebuild(&current.plans[0], snapshot).expect("reference rollup");
    assert_eq!(&after, reference.all());
}
