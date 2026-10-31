//! Verifies retries use the shared application plan mutation boundary.

use super::*;

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "The regression follows retry state and retained failure evidence end to end."
)]
/// Retries a failed node with exact plan revision and retained attempt history.
fn application_retry_is_revision_checked_and_preserves_failure() {
    let service = ApplicationService::new(Fixture);
    let context = session_context();
    let session_id = EntityId::from_bytes([131; 16]);
    service
        .start_agent(&context, session_id, "retry plan")
        .expect("seed session");
    let state = service
        .read_session(
            &context,
            &SessionRequest {
                session: session_id,
            },
        )
        .expect("session");
    let plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session: session_id,
                anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: state.branches[0].id,
                },
                goal: "retry failed work".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "work".to_owned(),
                    parent_index: None,
                    title: "Work".to_owned(),
                    purpose: "Complete the work".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("create plan");
    let running = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session: session_id,
                plan: plan.id,
                expected_revision: plan.revision,
            },
        )
        .expect("start plan");
    let node_id = running.nodes[0].id;
    let branch_id = EntityId::from_bytes([132; 16]);
    let claim_id = EntityId::from_bytes([133; 16]);
    let failure_id = EntityId::from_bytes([134; 16]);
    let mut failed = service
        .read_session(
            &context,
            &SessionRequest {
                session: session_id,
            },
        )
        .expect("running session");
    let expected = failed.plans[0].revision;
    failed.plans[0]
        .start_node(node_id, branch_id, claim_id, "worker", &[])
        .expect("start node");
    let mut branch =
        scriberig_session::Branch::new(branch_id, "failed attempt").expect("worker branch");
    branch.parent = Some(state.branches[0].id);
    branch.result = Some(scriberig_session::BranchResult {
        id: failure_id,
        branch: branch_id,
        status: scriberig_session::BranchStatus::Abandoned,
        summary: Some("worker failed".to_owned()),
        result_kind: None,
        candidate_context: None,
        references: Vec::new(),
        result_handles: Vec::new(),
        touched: Vec::new(),
        produced_changes: Vec::new(),
        validations: Vec::new(),
        unresolved_questions: Vec::new(),
    });
    failed.branches.push(branch);
    service
        .compare_and_swap_plan(&context, &failed, plan.id, expected)
        .expect("persist running node");

    let mut failed = service
        .read_session(
            &context,
            &SessionRequest {
                session: session_id,
            },
        )
        .expect("running worker");
    let expected = failed.plans[0].revision;
    let failure = failed.branches[1].result.clone().expect("failure result");
    failed.plans[0]
        .settle_workers(&[], &[(node_id, claim_id, Some(failure))], &[])
        .expect("fail node");
    service
        .compare_and_swap_plan(&context, &failed, plan.id, expected)
        .expect("persist failure");

    let failed_revision = failed.plans[0].revision;
    let retried = service
        .retry_plan_node(
            &context,
            &scriberig_app::PlanRetryRequest {
                session: session_id,
                plan: plan.id,
                node: node_id,
                expected_revision: failed_revision,
            },
        )
        .expect("retry node");
    assert_eq!(
        retried.nodes[0].status,
        scriberig_session::PlanNodeStatus::Ready
    );
    assert_eq!(retried.nodes[0].branch, None);
    assert_eq!(
        retried.nodes[0]
            .last_failure
            .as_ref()
            .map(|failure| failure.reference.entity),
        Some(failure_id)
    );
    assert!(
        retried.attempt_history(node_id).unwrap()[0]
            .failure
            .is_some()
    );
    assert!(
        service
            .retry_plan_node(
                &context,
                &scriberig_app::PlanRetryRequest {
                    session: session_id,
                    plan: plan.id,
                    node: node_id,
                    expected_revision: failed_revision,
                },
            )
            .is_err()
    );
}
