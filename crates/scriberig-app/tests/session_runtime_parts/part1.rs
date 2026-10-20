//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies bootstrap agent execution starts without semantic project scope.
#[test]
fn bootstrap_agent_execution_starts_without_semantic_project_scope() {
    let service = ApplicationService::new(Fixture);
    let session = EntityId::from_bytes([9; 16]);
    let context = session_context();
    let handle = service
        .begin_agent_execution(
            &context,
            &AgentCommandRequest {
                session,
                branch: None,
                anchor: None,
                prompt: "Onboard the current project.".to_owned(),
                selection: None,
            },
        )
        .expect("bootstrap operation accepted without project scope");
    assert_eq!(handle.session, session);
    assert_eq!(
        service.operations().get(&handle.operation).unwrap().status,
        scriberig_app::OperationStatus::Running
    );
    let session = service
        .read_session(&session_context(), &SessionRequest { session })
        .expect("durable session");
    assert_eq!(session.branches.len(), 1);
    assert_eq!(
        session.branches[0].turns[0].human.as_ref().unwrap().text,
        "Onboard the current project."
    );
}

/// Starts a new session with app allocated identities and continues exact history.
#[test]
fn shared_new_session_and_exact_continuation_return_durable_handles() {
    let service = ApplicationService::new(Fixture);
    let context = session_context();
    let accepted = service
        .start_new_session_agent(&context, "first request")
        .expect("new session accepted");
    let initial = service
        .read_session(
            &context,
            &SessionRequest {
                session: accepted.session,
            },
        )
        .expect("initial session persisted");
    assert_eq!(initial.branches[0].id, accepted.root_branch);
    assert_eq!(
        initial.branches[0].turns[0]
            .human
            .as_ref()
            .expect("initial human input")
            .text,
        "first request"
    );
    let first_handle = service
        .begin_agent_execution(
            &session_context(),
            &AgentCommandRequest {
                session: accepted.session,
                branch: Some(accepted.root_branch),
                anchor: None,
                prompt: "second request".to_owned(),
                selection: None,
            },
        )
        .expect("second turn accepted");
    service
        .complete_local_agent(&first_handle, "second response")
        .expect("second response persisted");
    let state = service
        .read_session(
            &context,
            &SessionRequest {
                session: accepted.session,
            },
        )
        .expect("session state");
    let anchor = scriberig_session::ContinuationAnchor::AssistantOutput {
        branch: first_handle.root_branch,
        turn: state
            .branches
            .iter()
            .find(|branch| branch.id == first_handle.root_branch)
            .expect("continuation branch")
            .turns[0]
            .id,
    };
    let accepted_continuation = service
        .continue_agent_at(&context, accepted.session, anchor, "historical follow-up")
        .expect("exact continuation accepted");
    assert_ne!(accepted_continuation.operation, accepted.operation);
    assert_eq!(accepted_continuation.session, accepted.session);
    let state = service
        .read_session(
            &context,
            &SessionRequest {
                session: accepted.session,
            },
        )
        .expect("continued session");
    assert_eq!(state.branches.len(), 3);
    assert_eq!(state.branches[0].turns.len(), 1);
    let branch = state
        .branches
        .iter()
        .find(|branch| {
            branch
                .continuation
                .is_some_and(|origin| origin.anchor == anchor)
        })
        .expect("historical continuation branch");
    assert_eq!(accepted_continuation.branch, branch.id);
    assert_eq!(branch.parent, Some(first_handle.root_branch));
    assert_eq!(
        branch.turns[0].human.as_ref().unwrap().text,
        "historical follow-up"
    );
}

/// Creates a durable plan graph and checks optimistic lifecycle mutations.
#[test]
#[allow(clippy::too_many_lines)]
fn application_plan_operations_validate_and_persist_structured_work() {
    let service = ApplicationService::new(Fixture).with_upstream_models(session_upstream_models());
    let context = session_context();
    let session = EntityId::from_bytes([101; 16]);
    service
        .start_agent(&context, session, "plan anchor")
        .expect("seed session");
    let state = service
        .read_session(&context, &SessionRequest { session })
        .expect("session");
    let branch = state.branches[0].id;
    let request = PlanCreateRequest {
        session,
        anchor: scriberig_session::ContinuationAnchor::BranchHead { branch },
        goal: "finish the feature".to_owned(),
        nodes: vec![
            PlanNodeDraft {
                key: "design".to_owned(),
                parent_index: None,
                title: "Design".to_owned(),
                purpose: "define the interface".to_owned(),
                expected_result: Some(scriberig_session::ExpectedResult {
                    kind: "semantic-summary".to_owned(),
                    requires_handle: true,
                }),
                semantic_scope: vec!["entity:feature".to_owned()],
                assignment: Some(scriberig_session::ModelAssignment {
                    provider_id: "local".to_owned(),
                    model_id: "native-a".to_owned(),
                    thinking_level: None,
                }),
            },
            PlanNodeDraft {
                key: "implement".to_owned(),
                parent_index: Some(0),
                title: "Implement".to_owned(),
                purpose: "apply the design".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: None,
            },
            PlanNodeDraft {
                key: "verify".to_owned(),
                parent_index: Some(0),
                title: "Verify".to_owned(),
                purpose: "check the integrated behavior".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: None,
            },
        ],
        dependencies: vec![(0, 2)],
    };
    let plan_events = service.operations().subscribe();
    let plan = service.create_plan(&context, &request).expect("plan");
    assert!(matches!(
        plan_events.try_recv().expect("settled plan event"),
        scriberig_app::ApplicationEvent::PlanActivity {
            session: event_session,
            plan: event_plan,
            activity_kind: scriberig_session::SessionActivityKind::PlanCreated,
            state: scriberig_session::SessionActivityState::Completed,
            ..
        } if event_session == session && event_plan == plan.id
    ));
    let mut cyclic = request.clone();
    cyclic.dependencies = vec![(0, 1), (1, 0)];
    assert!(service.create_plan(&context, &cyclic).is_err());
    let before_lifecycle = service
        .read_session(&context, &SessionRequest { session })
        .expect("session without invalid plan");
    assert_eq!(before_lifecycle.plans.len(), 1);
    assert_eq!(plan.status, scriberig_session::PlanStatus::Planned);
    assert_eq!(plan.nodes.len(), 3);
    assert_eq!(plan.session, Some(session));
    assert_eq!(plan.anchor, Some(request.anchor));
    assert_eq!(
        plan.nodes[1].parent,
        Some(scriberig_session::PlanNodeParent::Node(plan.nodes[0].id))
    );
    assert_eq!(plan.dependencies[0].prerequisite, plan.nodes[0].id);
    assert_eq!(plan.dependencies[0].dependent, plan.nodes[2].id);
    assert!(plan.dependencies[0].fulfilled_by.is_none());
    assert_eq!(
        plan.nodes[0].assignment.as_ref().unwrap().model_id,
        "native-a"
    );
    let assigned = service
        .set_plan_node_assignment(
            &context,
            &PlanAssignmentRequest {
                session,
                plan: plan.id,
                node: plan.nodes[1].id,
                expected_revision: plan.revision,
                assignment: Some(scriberig_session::ModelAssignment {
                    provider_id: "local".to_owned(),
                    model_id: "native-b".to_owned(),
                    thinking_level: Some("steady".to_owned()),
                }),
            },
        )
        .expect("assignment mutation");
    assert_eq!(
        assigned.nodes[1].assignment.as_ref().unwrap().model_id,
        "native-b"
    );
    assert!(assigned.history.iter().any(|revision| {
        revision.nodes.iter().any(|node| {
            node.id == assigned.nodes[1].id
                && node
                    .assignment
                    .as_ref()
                    .is_some_and(|assignment| assignment.model_id == "native-b")
        })
    }));
    let started = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: plan.id,
                expected_revision: assigned.revision,
            },
        )
        .expect("start plan");
    let coordinator_claim = started
        .coordinator_claim
        .clone()
        .expect("durable coordinator claim");
    let coordinator_operation = coordinator_claim
        .operation
        .clone()
        .expect("shared coordinator operation");
    assert_eq!(
        service
            .operations()
            .get(&coordinator_operation)
            .unwrap()
            .status,
        scriberig_app::OperationStatus::Running
    );
    let started_again = service
        .start_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: plan.id,
                expected_revision: started.revision,
            },
        )
        .expect("idempotent coordinator start");
    assert_eq!(started_again.revision, started.revision);
    assert_eq!(
        started_again.coordinator_claim,
        Some(coordinator_claim.clone())
    );
    let paused = service
        .pause_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: started.id,
                expected_revision: started_again.revision,
            },
        )
        .expect("pause plan");
    assert_eq!(paused.status, scriberig_session::PlanStatus::Paused);
    assert!(paused.coordinator_claim.is_none());
    assert_eq!(
        service
            .operations()
            .get(&coordinator_operation)
            .unwrap()
            .status,
        scriberig_app::OperationStatus::Cancelled
    );
    let resumed = service
        .resume_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: paused.id,
                expected_revision: paused.revision,
            },
        )
        .expect("resume coordinator");
    let resumed_claim = resumed
        .coordinator_claim
        .as_ref()
        .expect("resumed coordinator claim");
    assert!(resumed_claim.generation > coordinator_claim.generation);
    let resumed_operation = resumed_claim
        .operation
        .as_ref()
        .expect("resumed shared operation");
    assert_eq!(
        service.operations().get(resumed_operation).unwrap().status,
        scriberig_app::OperationStatus::Running
    );
    assert!(
        service
            .resume_plan(
                &context,
                &PlanLifecycleRequest {
                    session,
                    plan: resumed.id,
                    expected_revision: paused.revision,
                },
            )
            .is_err()
    );
    let stopped = service
        .stop_plan(
            &context,
            &PlanLifecycleRequest {
                session,
                plan: resumed.id,
                expected_revision: resumed.revision,
            },
        )
        .expect("stop plan");
    assert_eq!(stopped.status, scriberig_session::PlanStatus::Cancelled);
    assert!(stopped.coordinator_claim.is_none());
    assert_eq!(
        service.operations().get(resumed_operation).unwrap().status,
        scriberig_app::OperationStatus::Cancelled
    );
    let loaded = service
        .read_session(&context, &SessionRequest { session })
        .expect("persisted plan");
    assert_eq!(loaded.plans[0].revision, stopped.revision);
}

#[path = "part2.rs"]
mod part2;
