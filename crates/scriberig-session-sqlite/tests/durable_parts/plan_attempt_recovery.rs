//! Verifies retry history survives semantic plan persistence.

use super::*;
use scriberig_core::RootName;
use scriberig_store::{RevisionStore, RootStore, SnapshotStore};

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The regression follows a failed attempt and retry through one durable reload."
)]
/// Retains a failed attempt separately from its later successful retry.
fn semantic_retry_keeps_attempt_outcomes_across_reload() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-semantic-attempt-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([71; 16]);
    let root_id = EntityId::from_bytes([72; 16]);
    let plan_id = EntityId::from_bytes([73; 16]);
    let node_id = EntityId::from_bytes([74; 16]);
    let first_branch = EntityId::from_bytes([75; 16]);
    let first_claim = EntityId::from_bytes([76; 16]);
    let failure_id = EntityId::from_bytes([77; 16]);
    let second_branch = EntityId::from_bytes([78; 16]);
    let second_claim = EntityId::from_bytes([79; 16]);
    let success_id = EntityId::from_bytes([80; 16]);
    let store = SqliteSessionStore::open(&path).expect("open store");
    let mut plan = Plan::new(
        plan_id,
        session_id,
        scriberig_session::ContinuationAnchor::BranchHead { branch: root_id },
        "recover failed work",
    )
    .expect("plan");
    plan.add_node(PlanNode {
        id: node_id,
        parent: Some(scriberig_session::PlanNodeParent::Branch(root_id)),
        title: "perform task".to_owned(),
        purpose: "complete the requested task".to_owned(),
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
    plan.transition(scriberig_session::PlanStatus::Planned)
        .expect("plan");
    plan.start_with_coordinator_claim(EntityId::from_bytes([81; 16]), "coordinator", None)
        .expect("start plan");
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

    let mut first_attempt = store
        .load(session_id)
        .expect("load initial plan")
        .expect("session exists");
    let expected = first_attempt.plans[0].revision;
    first_attempt.plans[0]
        .start_node(node_id, first_branch, first_claim, "worker", &[])
        .expect("start first attempt");
    let mut first_branch_record = Branch::new(first_branch, "first attempt").expect("branch");
    first_branch_record.parent = Some(root_id);
    first_branch_record.result = Some(BranchResult {
        id: failure_id,
        branch: first_branch,
        status: BranchStatus::Abandoned,
        summary: Some("first attempt failed".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    first_attempt.branches.push(first_branch_record);
    store
        .compare_and_swap_plan(&first_attempt, plan_id, expected)
        .expect("persist first attempt");

    let mut failed_attempt = store
        .load(session_id)
        .expect("load first attempt")
        .expect("session exists");
    let expected = failed_attempt.plans[0].revision;
    let failure = failed_attempt.branches[1]
        .result
        .clone()
        .expect("failure result");
    failed_attempt.plans[0]
        .settle_workers(&[], &[(node_id, first_claim, Some(failure))], &[])
        .expect("settle first attempt as failed");
    store
        .compare_and_swap_plan(&failed_attempt, plan_id, expected)
        .expect("persist failed attempt");

    let mut retry = store
        .load(session_id)
        .expect("load failed attempt")
        .expect("session exists");
    let expected = retry.plans[0].revision;
    retry.plans[0]
        .retry_failed_node(node_id, &[])
        .expect("retry failed node");
    store
        .compare_and_swap_plan(&retry, plan_id, expected)
        .expect("persist retry");

    let mut second_attempt = store
        .load(session_id)
        .expect("load retry")
        .expect("session exists");
    let expected = second_attempt.plans[0].revision;
    second_attempt.plans[0]
        .start_node(node_id, second_branch, second_claim, "worker", &[])
        .expect("start second attempt");
    let mut second_branch_record = Branch::new(second_branch, "second attempt").expect("branch");
    second_branch_record.parent = Some(root_id);
    second_attempt.branches.push(second_branch_record);
    store
        .compare_and_swap_plan(&second_attempt, plan_id, expected)
        .expect("persist second attempt");

    let mut completed = store
        .load(session_id)
        .expect("load second attempt")
        .expect("session exists");
    let expected = completed.plans[0].revision;
    let success = BranchResult {
        id: success_id,
        branch: second_branch,
        status: BranchStatus::Completed,
        summary: Some("second attempt succeeded".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    completed.plans[0]
        .settle_workers(&[(node_id, second_claim, success.clone())], &[], &[success])
        .expect("complete second attempt");
    store
        .compare_and_swap_plan(&completed, plan_id, expected)
        .expect("persist successful retry");

    let reloaded = store
        .load(session_id)
        .expect("reload plan history")
        .expect("session exists");
    let attempts = reloaded.plans[0]
        .attempt_history(node_id)
        .expect("known node history");
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].branch, first_branch);
    assert_eq!(attempts[0].claim_generation, 1);
    assert_eq!(attempts[0].status, PlanNodeStatus::Failed);
    assert_eq!(
        attempts[0]
            .failure
            .as_ref()
            .map(|reference| reference.reference.entity),
        Some(failure_id)
    );
    assert_eq!(attempts[1].branch, second_branch);
    assert_eq!(attempts[1].claim_generation, 2);
    assert_eq!(attempts[1].status, PlanNodeStatus::Completed);
    assert_eq!(attempts[1].result, Some(success_id));
    assert_eq!(attempts[1].failure, None);

    let semantic_store =
        scriberig_store_sqlite::SqliteStore::open(&path).expect("open semantic store");
    let root = RootName::new(format!("scriberig-plan-{session_id}-{plan_id}")).expect("plan root");
    let snapshot = semantic_store.read_root(&root).expect("current plan root");
    let objects = semantic_store
        .iter(snapshot)
        .expect("read current semantic plan")
        .into_iter()
        .map(|(entity, revision_id)| {
            let revision = semantic_store
                .get(revision_id)
                .expect("read semantic revision")
                .expect("semantic revision exists");
            scriberig_session::PlanSemanticRevision {
                entity,
                object_type: revision.object_type().clone(),
                revision: revision.id(),
                payload: revision.payload().to_vec(),
            }
        })
        .collect();
    let semantic_snapshot = scriberig_session::PlanSemanticSnapshot {
        plan: plan_id,
        snapshot: Some(snapshot),
        objects,
    };
    let semantic_attempts = semantic_snapshot.attempts().expect("semantic attempts");

    assert_eq!(semantic_attempts.len(), 2);
    assert_eq!(semantic_attempts[0].branch, first_branch);
    assert_eq!(semantic_attempts[0].status, PlanNodeStatus::Failed);
    assert_eq!(
        semantic_attempts[0]
            .failure
            .as_ref()
            .map(|reference| reference.reference.entity),
        Some(failure_id)
    );
    assert_eq!(semantic_attempts[1].branch, second_branch);
    assert_eq!(semantic_attempts[1].status, PlanNodeStatus::Completed);
    assert_eq!(semantic_attempts[1].result, Some(success_id));
    drop(semantic_store);
    drop(store);
    let _ = std::fs::remove_file(path);
}
