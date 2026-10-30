//! Covers additional focused cases for the parent test module.

use super::*;

/// Appends coordinator-discovered work while a plan is already running.
#[test]
fn running_plan_accepts_next_step_with_one_mutation_revision() {
    let mut plan = plan_fixture();
    plan.transition(PlanStatus::Running).expect("run plan");
    let prior_revision = plan.revision;
    let prior_nodes = plan.nodes.len();
    let next_step = plan_node(25, plan.root_branch, "newly discovered work");

    plan.add_next_step(next_step.clone())
        .expect("append next step");

    assert_eq!(plan.revision, prior_revision + 1);
    assert_eq!(plan.nodes.len(), prior_nodes + 1);
    assert_eq!(plan.nodes.last().unwrap().id, next_step.id);
    assert_eq!(plan.nodes.last().unwrap().status, PlanNodeStatus::Ready);
    assert_eq!(
        plan.history
            .last()
            .and_then(|revision| revision.reason.as_deref()),
        Some("ordinary next step added")
    );
}

/// Keeps failed worker provenance while retrying the same semantic node.
#[test]
fn failed_plan_node_retries_with_a_new_branch_and_claim_generation() {
    let mut plan = plan_fixture();
    let node = plan.nodes[0].id;
    let first_branch = EntityId::from_bytes([220; 16]);
    let first_claim = EntityId::from_bytes([221; 16]);
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([222; 16]), "coordinator")
        .expect("claim coordinator");
    plan.start_node(node, first_branch, first_claim, "worker", &[])
        .expect("start first attempt");
    let failure_id = EntityId::from_bytes([228; 16]);
    let mut failure = branch_result(failure_id, first_branch, None);
    failure.status = BranchStatus::Abandoned;
    failure.result_kind = Some("worker_failure".to_owned());
    plan.settle_workers(
        &[],
        &[(node, first_claim, Some(failure.clone()))],
        std::slice::from_ref(&failure),
    )
    .expect("fail first attempt");
    let failed_revision = plan.revision;
    assert_eq!(
        plan.start_node(
            node,
            EntityId::from_bytes([226; 16]),
            EntityId::from_bytes([227; 16]),
            "worker",
            &[],
        ),
        Err(PlanError::InvalidTransition)
    );

    plan.retry_failed_node(node, &[])
        .expect("retry failed node");

    let retried = plan.nodes.iter().find(|entry| entry.id == node).unwrap();
    assert_eq!(retried.status, PlanNodeStatus::Ready);
    assert_eq!(retried.branch, None);
    assert_eq!(retried.claim_generation, 1);
    assert_eq!(
        retried
            .last_failure
            .as_ref()
            .map(|item| item.reference.entity),
        Some(failure_id)
    );
    assert!(plan.history.iter().any(|revision| {
        revision.revision == failed_revision
            && revision.nodes.iter().any(|entry| {
                entry.id == node
                    && entry.status == PlanNodeStatus::Failed
                    && entry.branch == Some(first_branch)
            })
    }));

    let second_branch = EntityId::from_bytes([223; 16]);
    let second_claim = EntityId::from_bytes([224; 16]);
    plan.start_node(node, second_branch, second_claim, "worker", &[])
        .expect("start second attempt");
    assert_eq!(
        plan.nodes
            .iter()
            .find(|entry| entry.id == node)
            .unwrap()
            .claim_generation,
        2
    );
    let attempts = plan.attempt_history(node).expect("node attempt history");
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].branch, first_branch);
    assert_eq!(attempts[0].status, PlanNodeStatus::Failed);
    assert_eq!(attempts[0].claim, Some(first_claim));
    assert_eq!(
        attempts[0]
            .failure
            .as_ref()
            .map(|item| item.reference.entity),
        Some(failure_id)
    );
    assert_eq!(attempts[1].branch, second_branch);
    assert_eq!(attempts[1].status, PlanNodeStatus::Running);
    assert_eq!(attempts[1].claim, Some(second_claim));
}

/// Rejects retry when the node's exact dependency result is unavailable.
#[test]
fn failed_plan_node_retry_requires_completed_dependency_results() {
    let mut plan = plan_fixture();
    let prerequisite = plan.nodes[0].id;
    let failed = plan.nodes[1].id;
    plan.add_dependency(prerequisite, failed)
        .expect("dependency");
    let target = plan
        .nodes
        .iter_mut()
        .find(|node| node.id == failed)
        .unwrap();
    target.status = PlanNodeStatus::Failed;
    target.branch = Some(EntityId::from_bytes([225; 16]));

    assert_eq!(
        plan.retry_failed_node(failed, &[]),
        Err(PlanError::InvalidTransition)
    );
    assert_eq!(
        plan.nodes
            .iter()
            .find(|node| node.id == failed)
            .unwrap()
            .status,
        PlanNodeStatus::Failed
    );
}

/// Recovers a failed plan only after its worker claims have settled.
#[test]
fn failed_plan_recovers_to_paused_for_explicit_repair() {
    let mut plan = plan_fixture();
    plan.status = PlanStatus::Failed;

    plan.recover_failed_plan().expect("recover failed plan");

    assert_eq!(plan.status, PlanStatus::Paused);
    assert_eq!(plan.history.last().unwrap().status, PlanStatus::Paused);
}

/// Repairs a failed plan through the explicit pause and validated replan path.
#[test]
fn failed_plan_recovery_allows_validated_replan() {
    let mut plan = plan_fixture();
    plan.status = PlanStatus::Failed;
    plan.recover_failed_plan().expect("recover failed plan");
    let original_revision = plan.revision;
    let addition = plan_node(35, plan.root_branch, "repair work");

    plan.replan(
        vec![addition.clone()],
        Vec::new(),
        &[],
        &[],
        &[],
        &[],
        "repair failed plan",
    )
    .expect("validated replan");

    assert_eq!(plan.status, PlanStatus::Paused);
    assert_eq!(plan.revision, original_revision + 1);
    assert!(plan.nodes.iter().any(|node| node.id == addition.id));
}

/// Keeps completed and cancelled plans terminal for ordinary node mutation.
#[test]
fn completed_and_cancelled_plans_reject_ordinary_mutations() {
    for status in [PlanStatus::Completed, PlanStatus::Cancelled] {
        let mut plan = plan_fixture();
        plan.status = status;

        assert_eq!(
            plan.add_node(plan_node(36, plan.root_branch, "late work")),
            Err(PlanError::TerminalPlan)
        );
    }
}

/// Verifies stale plan claim recovery is deterministic and blocks dependents.
#[test]
fn stale_plan_claim_recovery_is_deterministic_and_blocks_dependents() {
    let mut plan = plan_fixture();
    let producer = plan.nodes[0].id;
    let dependent = plan.nodes[2].id;
    plan.add_dependency(producer, dependent)
        .expect("dependency");
    let branch = EntityId::from_bytes([210; 16]);
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([211; 16]), "gone-coordinator")
        .expect("coordinator");
    plan.start_node(
        producer,
        branch,
        EntityId::from_bytes([212; 16]),
        "gone-worker",
        &[],
    )
    .expect("worker");
    assert_eq!(plan.recover_stale_claims(|_| false), Ok(2));
    assert!(plan.coordinator_claim.is_none());
    assert_eq!(plan.nodes[0].status, PlanNodeStatus::Failed);
    assert_eq!(plan.nodes[0].branch, Some(branch));
    assert_eq!(plan.nodes[0].claim, None);
    assert_eq!(plan.dependencies[0].fulfilled_by, None);
    assert_eq!(plan.nodes[2].status, PlanNodeStatus::Planned);
    assert!(!plan.ready_nodes_from_results(&[]).contains(&dependent));
}

/// Prevents a recovered stale coordinator from dispatching ready work.
#[test]
fn stale_coordinator_cannot_dispatch_after_recovery() {
    let mut plan = plan_fixture();
    let node = plan.nodes[0].id;
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([213; 16]), "stale-runtime")
        .expect("claim coordinator");

    assert_eq!(plan.recover_stale_claims(|_| false), Ok(1));
    let recovered_revision = plan.revision;
    assert!(plan.coordinator_claim.is_none());
    assert_eq!(
        plan.start_node(
            node,
            EntityId::from_bytes([214; 16]),
            EntityId::from_bytes([215; 16]),
            "stale-runtime",
            &[],
        ),
        Err(PlanError::InvalidTransition)
    );
    assert_eq!(plan.revision, recovered_revision);
    assert_eq!(plan.nodes[0].status, PlanNodeStatus::Ready);
}

/// Requires exact result class and handle evidence before releasing dependents.
#[test]
fn plan_readiness_requires_explicit_result_contract() {
    let root = EntityId::from_bytes([80; 16]);
    let source = EntityId::from_bytes([81; 16]);
    let dependent = EntityId::from_bytes([82; 16]);
    let branch = EntityId::from_bytes([83; 16]);
    let result_id = EntityId::from_bytes([84; 16]);
    let mut plan = Plan {
        id: EntityId::from_bytes([85; 16]),
        session: None,
        anchor: None,
        source_snapshot: None,
        goal: "contract test".to_owned(),
        revision: 0,
        status: PlanStatus::Planned,
        root_branch: root,
        nodes: Vec::new(),
        dependencies: Vec::new(),
        summaries: Vec::new(),
        evidence: Vec::new(),
        gates: Vec::new(),
        coordinator: None,
        coordinator_cursor: None,
        coordinator_claim: None,
        coordinator_generation: 0,
        root_nodes: Vec::new(),
        history: Vec::new(),
    };
    let mut producer = plan_node(81, root, "producer");
    producer.expected_result = Some(scriberig_session::ExpectedResult {
        kind: "semantic-summary".to_owned(),
        requires_handle: true,
    });
    plan.add_node(producer).expect("producer");
    plan.add_node(plan_node(82, root, "consumer"))
        .expect("consumer");
    plan.add_dependency(source, dependent).expect("dependency");
    plan.transition(PlanStatus::Running).expect("run plan");
    plan.claim_coordinator(EntityId::from_bytes([86; 16]), "coordinator")
        .expect("claim coordinator");
    let claim_id = EntityId::from_bytes([87; 16]);
    plan.start_node(source, branch, claim_id, "worker", &[])
        .expect("start producer");
    let wrong_class = branch_result(result_id, branch, Some("text"));
    assert_eq!(
        plan.complete_node(source, claim_id, &wrong_class),
        Err(PlanError::ResultMismatch)
    );
    let valid_result = branch_result(result_id, branch, Some("semantic-summary"));
    plan.complete_node(source, claim_id, &valid_result)
        .expect("publish valid result");
    assert_eq!(plan.ready_nodes_from_results(&[]), Vec::<EntityId>::new());
    assert_eq!(
        plan.ready_nodes_from_results(std::slice::from_ref(&valid_result)),
        vec![dependent]
    );
    let mut failed = valid_result;
    failed.status = BranchStatus::Abandoned;
    assert_eq!(
        plan.ready_nodes_from_results(&[failed]),
        Vec::<EntityId>::new()
    );
}

#[path = "part1/replan_completion.rs"]
mod replan_completion;
pub(super) use replan_completion::plan_fixture;

#[path = "part2.rs"]
mod part2;
