//! Verifies failed-plan recovery and repair through shared application APIs.

use super::*;

/// Recovers and repairs a failed plan through revision-checked app operations.
#[test]
fn failed_plan_recovery_and_replan_use_shared_mutations() {
    let service = ApplicationService::new(Fixture).with_upstream_models(session_upstream_models());
    let context = session_context();
    let session = EntityId::from_bytes([241; 16]);
    service
        .start_agent(&context, session, "failed plan anchor")
        .expect("seed session");
    let state = service
        .read_session(&context, &SessionRequest { session })
        .expect("read session");
    let mut plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead {
                    branch: state.branches[0].id,
                },
                goal: "recover failed work".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "first".to_owned(),
                    parent_index: None,
                    title: "First".to_owned(),
                    purpose: "initial work".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("create plan");
    plan = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
            },
        )
        .expect("start plan");

    let mut state = service
        .read_session(&context, &SessionRequest { session })
        .expect("read running plan");
    state.plans[0]
        .transition(scriberig_session::PlanStatus::Failed)
        .expect("record plan failure");
    SessionBackend::compare_and_swap_plan(&service, &context, &state, plan.id, plan.revision)
        .expect("persist failed plan state");
    plan = service
        .mutate_plan(
            &context,
            &PlanMutationRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision + 1,
                mutation: PlanMutation::RecoverFailedPlan,
            },
        )
        .expect("recover failed plan");
    assert_eq!(plan.status, scriberig_session::PlanStatus::Paused);

    plan = service
        .resume_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision,
            },
        )
        .expect("resume repaired plan");
    let mut state = service
        .read_session(&context, &SessionRequest { session })
        .expect("read resumed plan");
    state.plans[0]
        .transition(scriberig_session::PlanStatus::Failed)
        .expect("record another plan failure");
    SessionBackend::compare_and_swap_plan(&service, &context, &state, plan.id, plan.revision)
        .expect("persist second failed plan state");

    plan = service
        .replan(
            &context,
            &scriberig_app::PlanReplanRequest {
                session,
                plan: plan.id,
                expected_revision: plan.revision + 1,
                additions: vec![PlanNodeDraft {
                    key: "repair".to_owned(),
                    parent_index: None,
                    title: "Repair".to_owned(),
                    purpose: "repair the failed plan".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                }],
                dependencies: Vec::new(),
                removed_dependencies: Vec::new(),
                superseded: Vec::new(),
                assignment_changes: Vec::new(),
                preserved_terminal_nodes: Vec::new(),
                reason: "explicit recovery repair".to_owned(),
            },
        )
        .expect("replan failed plan");
    assert_eq!(plan.status, scriberig_session::PlanStatus::Paused);
    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(
        plan.history.last().and_then(|item| item.reason.as_deref()),
        Some("explicit recovery repair")
    );
}
