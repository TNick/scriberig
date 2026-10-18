//! Verifies application-owned fallback session and agent state.
#[path = "session_runtime_support.rs"]
mod support;
use support::{Fixture, session_context};

use scriberig_app::{
    AgentCommandRequest, AgentConsoleBackend, ApplicationContext, ApplicationError,
    ApplicationService, CredentialRef, ModelCapabilities, ModelDescriptor, ModelId, ModelLibrary,
    ModelProfile, SessionBackend, SessionModelSelectionRequest, SessionRequest,
    SessionThinkingSelectionRequest, ThinkingLevelDescriptor, ThinkingLevelId,
};
use scriberig_core::EntityId;

/// Creates model profiles with independent thinking-level capabilities.
///
/// # Returns
/// A validated model library used by selection tests.
///
/// # Panics
/// Panics when the static fixture violates the model library contract.
fn session_model_library() -> ModelLibrary {
    let first = ModelDescriptor {
        id: ModelId::new("model-a").expect("model id"),
        display_name: "Model A".to_owned(),
        provider_profile: "local".to_owned(),
        provider_model_id: "native-a".to_owned(),
        capabilities: ModelCapabilities::default(),
        thinking_levels: vec![
            ThinkingLevelDescriptor {
                id: ThinkingLevelId::new("careful").expect("thinking id"),
                label: "Careful".to_owned(),
                description: None,
                order: 0,
            },
            ThinkingLevelDescriptor {
                id: ThinkingLevelId::new("extended").expect("thinking id"),
                label: "Extended".to_owned(),
                description: None,
                order: 1,
            },
        ],
        default_thinking_level: Some(ThinkingLevelId::new("careful").expect("thinking id")),
    };
    let second = ModelDescriptor {
        id: ModelId::new("model-b").expect("model id"),
        display_name: "Model B".to_owned(),
        provider_profile: "local".to_owned(),
        provider_model_id: "native-b".to_owned(),
        capabilities: ModelCapabilities::default(),
        thinking_levels: vec![ThinkingLevelDescriptor {
            id: ThinkingLevelId::new("steady").expect("thinking id"),
            label: "Steady".to_owned(),
            description: None,
            order: 0,
        }],
        default_thinking_level: Some(ThinkingLevelId::new("steady").expect("thinking id")),
    };
    let profiles = vec![
        ModelProfile {
            id: "profile-a".to_owned(),
            display_name: "Model A".to_owned(),
            model: first.id.clone(),
            connection: "local".to_owned(),
            credential: Some(CredentialRef {
                id: "secret-ref-a".to_owned(),
            }),
            default_thinking_level: None,
            enabled: true,
        },
        ModelProfile {
            id: "profile-b".to_owned(),
            display_name: "Model B".to_owned(),
            model: second.id.clone(),
            connection: "local".to_owned(),
            credential: None,
            default_thinking_level: None,
            enabled: true,
        },
    ];
    ModelLibrary::new(vec![first, second], profiles).expect("model library")
}

/// Verifies session model operations validate thinking levels and reject stale writes.
#[test]
fn session_model_selection_uses_library_and_mutation_revision() {
    let service = ApplicationService::new(Fixture).with_model_library(session_model_library());
    let context = session_context();
    let session = EntityId::from_bytes([24; 16]);
    service
        .start_agent(&context, session, "seed session")
        .expect("create session");

    let selected = service
        .set_session_model(
            &context,
            &SessionModelSelectionRequest {
                session,
                expected_revision: 0,
                model_profile: "profile-a".to_owned(),
            },
        )
        .expect("select first model");
    assert_eq!(selected.revision, 1);
    assert_eq!(selected.model.as_deref(), Some("model-a"));
    assert_eq!(selected.thinking_level.as_deref(), Some("careful"));

    let selected = service
        .set_session_thinking_level(
            &context,
            &SessionThinkingSelectionRequest {
                session,
                expected_revision: 1,
                thinking_level: "extended".to_owned(),
            },
        )
        .expect("select advertised thinking level");
    assert_eq!(selected.revision, 2);

    let selected = service
        .set_session_model(
            &context,
            &SessionModelSelectionRequest {
                session,
                expected_revision: 2,
                model_profile: "profile-b".to_owned(),
            },
        )
        .expect("switch model and fall back");
    assert_eq!(selected.revision, 3);
    assert_eq!(selected.thinking_level.as_deref(), Some("steady"));

    let conflict = service
        .set_session_model(
            &context,
            &SessionModelSelectionRequest {
                session,
                expected_revision: 2,
                model_profile: "profile-a".to_owned(),
            },
        )
        .expect_err("stale selection must not overwrite current state");
    assert_eq!(conflict.category(), "conflict");
    assert_eq!(
        service
            .read_session(&context, &SessionRequest { session })
            .expect("read selected session")
            .execution,
        selected
    );
    assert_eq!(
        service
            .operations()
            .events()
            .iter()
            .filter(|event| matches!(event, scriberig_app::ApplicationEvent::SessionChanged { session: changed } if *changed == session))
            .count(),
        3
    );
    let selection_events = service
        .operations()
        .events()
        .into_iter()
        .filter_map(|event| match event {
            scriberig_app::ApplicationEvent::SessionSelectionChanged {
                session: changed,
                activity_kind,
                summary,
            } if changed == session => Some((activity_kind, summary)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(selection_events.len(), 5);
    assert!(selection_events.iter().any(|(kind, summary)| {
        kind == "modelSelection" && summary == "Model selected: Model A"
    }));
    assert!(selection_events.iter().any(|(kind, summary)| {
        kind == "thinkingSelection" && summary == "Thinking level selected: steady"
    }));
    let available = service
        .agent_model_library(&context)
        .expect("authorized model library");
    assert_eq!(available.profiles.len(), 2);
    assert!(!format!("{available:?}").contains("resolved-secret"));
}

/// Verifies model selection fails without the model library capability.
#[test]
fn model_library_read_requires_agent_configuration_capability() {
    let service = ApplicationService::new(Fixture);

    let error = service
        .agent_model_library(&ApplicationContext::empty())
        .expect_err("model library must be authorized");

    assert_eq!(error.category(), "permission_denied");
}

/// Verifies local agent acceptance creates and updates a session branch.
#[test]
fn fallback_agent_acceptance_retains_session_activity() {
    let service = ApplicationService::new(Fixture);
    let session = EntityId::from_bytes([1; 16]);
    let context = session_context();
    let first = service
        .start_agent(&context, session, "first")
        .expect("first agent operation");
    let state = service
        .read_session(
            &session_context(),
            &scriberig_app::SessionRequest { session },
        )
        .expect("session state");
    assert_eq!(state.branches.len(), 1);
    assert_eq!(state.branches[0].turns.len(), 1);
    assert_eq!(
        state.branches[0].turns[0].human.as_ref().unwrap().text,
        "first"
    );
    assert_eq!(
        state.branches[0].turns[0].agent.as_ref().unwrap().text,
        "first"
    );
    assert!(state.branches[0].result.is_some());
    let first_record = service.operations().get(&first).unwrap();
    assert_eq!(
        first_record.status,
        scriberig_app::OperationStatus::Completed
    );
    assert_eq!(
        first_record.agent_result.as_ref().unwrap().final_output,
        "first"
    );
    assert_eq!(first_record.kind, scriberig_app::OperationKind::Agent);

    let branch = state.branches[0].id;
    let second = service
        .continue_agent(&context, session, branch, "second")
        .expect("continuation operation");
    assert_ne!(first, second);
    let state = service
        .read_session(
            &session_context(),
            &scriberig_app::SessionRequest { session },
        )
        .expect("continued session state");
    assert_eq!(state.branches.len(), 2);
    assert_eq!(state.branches[0].turns.len(), 1);
    assert_eq!(state.branches[1].parent, Some(branch));
    assert_eq!(state.branches[1].turns.len(), 1);
    assert_eq!(
        state.branches[1].turns[0]
            .human
            .as_ref()
            .expect("continuation input")
            .text,
        "second"
    );
}

/// Verifies authorized bootstrap execution can create a durable operation without project scope.
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

/// Verifies an agent command cannot bypass the application capability boundary.
#[test]
fn agent_submission_without_capability_is_denied() {
    let service = ApplicationService::new(Fixture);
    let error = service
        .start_agent(
            &ApplicationContext::empty(),
            EntityId::from_bytes([4; 16]),
            "denied",
        )
        .expect_err("agent capability must be required");

    assert!(matches!(
        error,
        ApplicationError::PermissionDenied { action, .. } if action == "agent_start"
    ));
}

/// Verifies session-tree reads cannot bypass the application capability boundary.
#[test]
fn session_read_without_capability_is_denied() {
    let service = ApplicationService::new(Fixture);
    let error = service
        .read_session(
            &ApplicationContext::empty(),
            &SessionRequest {
                session: EntityId::from_bytes([5; 16]),
            },
        )
        .expect_err("session capability must be required");

    assert!(matches!(
        error,
        ApplicationError::PermissionDenied { action, .. } if action == "session_branch_tree"
    ));
}

/// Verifies failed child execution is retained as an abandoned branch result.
#[test]
fn failed_local_child_agent_is_retained() {
    let service = ApplicationService::new(Fixture);
    let root = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([31; 16]),
            branch: None,
            prompt: "root".to_owned(),
            selection: None,
        })
        .expect("root");
    let child = service
        .start_local_child_agent(
            root.session,
            root.root_branch,
            "child",
            "inspect",
            "child work",
        )
        .expect("child");

    service
        .fail_local_child_agent(&child, "provider failed")
        .expect("failure retained");

    let state = service
        .read_session(
            &session_context(),
            &SessionRequest {
                session: root.session,
            },
        )
        .expect("session");
    let failed = state
        .branches
        .iter()
        .find(|branch| branch.id == child.child_branch)
        .expect("failed child");
    assert_eq!(failed.status, scriberig_session::BranchStatus::Abandoned);
    assert_eq!(failed.outcome.as_deref(), Some("provider failed"));
    assert_eq!(
        service.operations().get(&child.operation).unwrap().status,
        scriberig_app::OperationStatus::Failed
    );
}
