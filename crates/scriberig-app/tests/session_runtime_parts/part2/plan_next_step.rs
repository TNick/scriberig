//! Verifies ordinary work can be appended to a running durable plan.

use super::*;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The regression verifies valid and stale next-step mutations together."
)]
/// Appends a next step at an exact running-plan revision.
fn application_next_step_requires_running_plan_and_current_revision() {
    let service = ApplicationService::new(Fixture);
    let context = session_context();
    let session = EntityId::from_bytes([141; 16]);
    service
        .start_agent(&context, session, "next-step plan")
        .expect("seed session");
    let state = service
        .read_session(&context, &SessionRequest { session })
        .expect("session");
    let root = state.branches[0].id;
    let plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead { branch: root },
                goal: "evolve running work".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "first".to_owned(),
                    parent_index: None,
                    title: "First step".to_owned(),
                    purpose: "complete original work".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("create plan");
    let started = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
            },
        )
        .expect("start plan");
    let original_node = started.nodes[0].id;
    let worker_branch = EntityId::from_bytes([142; 16]);
    let worker_claim = EntityId::from_bytes([143; 16]);
    let result = scriberig_session::BranchResult {
        id: EntityId::from_bytes([144; 16]),
        branch: worker_branch,
        status: scriberig_session::BranchStatus::Completed,
        summary: Some("original work completed".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: vec!["original-result".to_owned()],
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    };
    let mut completed_session = service
        .read_session(&context, &SessionRequest { session })
        .expect("running session");
    let expected_revision = completed_session.plans[0].revision;
    completed_session.plans[0]
        .start_node(original_node, worker_branch, worker_claim, "worker", &[])
        .expect("claim original work");
    service
        .compare_and_swap_plan(&context, &completed_session, plan.id, expected_revision)
        .expect("persist worker claim");
    let mut completed_session = service
        .read_session(&context, &SessionRequest { session })
        .expect("claimed session");
    let expected_revision = completed_session.plans[0].revision;
    completed_session.plans[0]
        .complete_node(original_node, worker_claim, &result)
        .expect("complete original work");
    let mut worker =
        scriberig_session::Branch::new(worker_branch, "original attempt").expect("worker branch");
    worker.parent = Some(root);
    worker.status = scriberig_session::BranchStatus::Completed;
    worker.result = Some(result.clone());
    completed_session.branches.push(worker);
    service
        .compare_and_swap_plan(&context, &completed_session, plan.id, expected_revision)
        .expect("persist completed work");
    let running = service
        .read_session(&context, &SessionRequest { session })
        .expect("reload running plan")
        .plans
        .into_iter()
        .find(|candidate| candidate.id == plan.id)
        .expect("plan remains running");
    let next = service
        .add_plan_next_step(
            &context,
            &PlanNextStepRequest {
                session,
                plan: plan.id,
                expected_revision: running.revision,
                title: "Newly discovered step".to_owned(),
                purpose: "perform the follow-up work".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: None,
            },
        )
        .expect("append next step");

    assert_eq!(next.revision, running.revision + 1);
    assert_eq!(next.nodes.len(), 2);
    assert_eq!(next.nodes[0].id, running.nodes[0].id);
    assert_eq!(next.nodes[0].title, running.nodes[0].title);
    assert_eq!(
        next.nodes[0].status,
        scriberig_session::PlanNodeStatus::Completed
    );
    assert_eq!(next.nodes[0].result, Some(result.id));
    assert_eq!(
        next.history
            .last()
            .and_then(|revision| revision.reason.as_deref()),
        Some("ordinary next step added")
    );
    assert_eq!(
        next.nodes[1].parent,
        Some(scriberig_session::PlanNodeParent::Branch(root))
    );
    assert_eq!(
        next.nodes[1].status,
        scriberig_session::PlanNodeStatus::Ready
    );
    assert!(
        service
            .add_plan_next_step(
                &context,
                &PlanNextStepRequest {
                    session,
                    plan: plan.id,
                    expected_revision: running.revision,
                    title: "Stale step".to_owned(),
                    purpose: "must not be accepted".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                },
            )
            .is_err()
    );
}
