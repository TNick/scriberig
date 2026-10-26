//! Holds additional focused scenarios for the parent test module.

use super::*;

/// Exercises plan creation, start, read, and assignment through the Rust client.
#[test]
fn remote_client_can_create_start_read_and_assign_plan() {
    let client = scriberig_client::Client::new(scriberig_server::InProcessTransport::new(
        ApplicationService::new(Fixture),
    ));
    let session = "ent_61616161616161616161616161616161";
    let started = client
        .start_agent_with_handles(session, "plan root")
        .expect("seed session");
    let root = started.branch.expect("root branch");
    let plan = client
        .create_plan(SessionPlanCreateRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            anchor: SessionContinuationAnchor::BranchHead { branch: root.id },
            goal: "remote plan".to_owned(),
            nodes: vec![SessionPlanNodeDraft {
                key: "node".to_owned(),
                parent_index: None,
                title: "Node".to_owned(),
                purpose: "perform remote work".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: None,
            }],
            dependencies: Vec::new(),
        })
        .expect("create through client");
    let running = client
        .plan_lifecycle(SessionPlanLifecycleRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: plan.id.clone(),
            expected_revision: plan.revision,
            action: SessionPlanAction::Start,
        })
        .expect("start through client");
    let appended = client
        .mutate_plan(SessionPlanMutationRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: running.id.clone(),
            expected_revision: running.revision,
            mutation: SessionPlanMutation::AddNextStep {
                node: SessionPlanNodeDraft {
                    key: "next".to_owned(),
                    parent_index: None,
                    title: "Next".to_owned(),
                    purpose: "continue remote work".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: None,
                },
            },
        })
        .expect("append through client");
    assert_eq!(appended.nodes.len(), 2);
    let paused = client
        .plan_lifecycle(SessionPlanLifecycleRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: appended.id.clone(),
            expected_revision: appended.revision,
            action: SessionPlanAction::Pause,
        })
        .expect("pause through client");
    let assigned = client
        .set_plan_assignment(SessionPlanAssignmentRequest {
            session: SessionReference {
                id: session.to_owned(),
            },
            plan: paused.id.clone(),
            node: paused.nodes[0].id.clone(),
            expected_revision: paused.revision,
            assignment: None,
        })
        .expect("assignment through client");
    let read = client.read_session(session).expect("read through client");
    let mut assigned_without_history = assigned;
    assigned_without_history.history.clear();
    assert_eq!(read.plans[0], assigned_without_history);
    let new_session = client
        .start_new_session_agent("new conversation", None)
        .expect("new session through client");
    let root = new_session.branch.expect("client root branch");
    let continued = client
        .continue_agent_at(
            new_session.session.id,
            SessionContinuationAnchor::BranchHead { branch: root.id },
            "exact follow-up",
            None,
        )
        .expect("exact continuation through client");
    assert!(continued.branch.is_some());
}
