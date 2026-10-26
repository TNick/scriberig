//! Verifies application-owned fallback session and agent state.
#[path = "session_runtime_support.rs"]
mod support;
use support::{Fixture, session_context};

#[path = "session_runtime_access_tests/mod.rs"]
mod access_tests;

use rpi_ai::{Api, Model, ThinkingLevel, ThinkingLevelMap};
use scriberig_app::{
    AgentCommandRequest, AgentConsoleBackend, ApplicationContext, ApplicationError,
    ApplicationService, CredentialRef, ModelCapabilities, ModelDescriptor, ModelId, ModelLibrary,
    ModelProfile, PlanAssignmentRequest, PlanCreateRequest, PlanLifecycleRequest, PlanNodeDraft,
    SessionBackend, SessionModelSelectionRequest, SessionRequest, SessionThinkingSelectionRequest,
    ThinkingLevelDescriptor, ThinkingLevelId,
};
use scriberig_core::EntityId;

/// Creates upstream runtime models corresponding to the safe test projection.
///
/// The provider models used by session selection tests.
/// # Returns
/// The operation result.
fn session_upstream_models() -> Vec<Model> {
    let mut model_b = Model::new(
        "native-b",
        "Model B",
        Api::AnthropicMessages,
        "local",
        "https://example.test",
    );
    model_b.thinking_level_map = Some(ThinkingLevelMap::from([
        (ThinkingLevel::Minimal, None),
        (ThinkingLevel::Low, Some("low".to_owned())),
        (ThinkingLevel::Medium, None),
        (ThinkingLevel::High, None),
    ]));
    vec![
        Model::new(
            "native-a",
            "Model A",
            Api::AnthropicMessages,
            "local",
            "https://example.test",
        ),
        model_b,
    ]
}

/// Creates model profiles with independent thinking-level capabilities.
///
/// A validated model library used by selection tests.
///
/// # Panics
/// Panics when the static fixture violates the model library contract.
/// # Returns
/// The operation result.
fn session_model_library() -> ModelLibrary {
    let first = ModelDescriptor {
        id: ModelId::new("model-a").expect("model id"),
        display_name: "Model A".to_owned(),
        provider_profile: "local".to_owned(),
        provider_model_id: "native-a".to_owned(),
        capabilities: ModelCapabilities::default(),
        thinking_levels: vec![
            ThinkingLevelDescriptor {
                id: ThinkingLevelId::new("medium").expect("thinking id"),
                label: "Careful".to_owned(),
                description: None,
                order: 0,
            },
            ThinkingLevelDescriptor {
                id: ThinkingLevelId::new("high").expect("thinking id"),
                label: "Extended".to_owned(),
                description: None,
                order: 1,
            },
        ],
        default_thinking_level: Some(ThinkingLevelId::new("medium").expect("thinking id")),
    };
    let second = ModelDescriptor {
        id: ModelId::new("model-b").expect("model id"),
        display_name: "Model B".to_owned(),
        provider_profile: "local".to_owned(),
        provider_model_id: "native-b".to_owned(),
        capabilities: ModelCapabilities::default(),
        thinking_levels: vec![ThinkingLevelDescriptor {
            id: ThinkingLevelId::new("low").expect("thinking id"),
            label: "Steady".to_owned(),
            description: None,
            order: 0,
        }],
        default_thinking_level: Some(ThinkingLevelId::new("low").expect("thinking id")),
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
            default_thinking_level: Some(ThinkingLevelId::new("medium").expect("thinking id")),
            enabled: true,
        },
        ModelProfile {
            id: "profile-b".to_owned(),
            display_name: "Model B".to_owned(),
            model: second.id.clone(),
            connection: "local".to_owned(),
            credential: None,
            default_thinking_level: Some(ThinkingLevelId::new("low").expect("thinking id")),
            enabled: true,
        },
    ];
    ModelLibrary::new(vec![first, second], profiles).expect("model library")
}

/// Verifies session model operations validate thinking levels and reject stale writes.
#[test]
fn session_model_selection_uses_library_and_mutation_revision() {
    let service = ApplicationService::new(Fixture)
        .with_model_library(session_model_library())
        .with_upstream_models(session_upstream_models());
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
                provider_id: "local".to_owned(),
                model_id: "native-a".to_owned(),
            },
        )
        .expect("select first model");
    assert_eq!(selected.revision, 1);
    assert_eq!(selected.provider_id.as_deref(), Some("local"));
    assert_eq!(selected.model_id.as_deref(), Some("native-a"));
    assert_eq!(selected.thinking_level.as_deref(), Some("medium"));

    let selected = service
        .set_session_thinking_level(
            &context,
            &SessionThinkingSelectionRequest {
                session,
                expected_revision: 1,
                thinking_level: "high".to_owned(),
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
                provider_id: "local".to_owned(),
                model_id: "native-b".to_owned(),
            },
        )
        .expect("switch model and fall back");
    assert_eq!(selected.revision, 3);
    assert_eq!(selected.thinking_level.as_deref(), Some("low"));

    let conflict = service
        .set_session_model(
            &context,
            &SessionModelSelectionRequest {
                session,
                expected_revision: 2,
                provider_id: "local".to_owned(),
                model_id: "native-a".to_owned(),
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
    assert_selection_events(&service, session);
    assert_replayed_selection_history(&service, &context, session, &selected);
    let available = service
        .agent_model_library(&context)
        .expect("authorized model library");
    assert_eq!(available.profiles.len(), 2);
    assert!(!format!("{available:?}").contains("resolved-secret"));
}

/// Verifies typed live selection events include model and fallback activity.
///
/// - `service`: Application service publishing the selection events.
/// - `session`: Session identity whose events are inspected.
fn assert_selection_events(service: &ApplicationService<Fixture>, session: EntityId) {
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
        *kind == scriberig_session::AgentActivityKind::ModelSelectionChanged
            && summary == "Model selected: Model A"
    }));
    assert!(selection_events.iter().any(|(kind, summary)| {
        *kind == scriberig_session::AgentActivityKind::ThinkingLevelChanged
            && summary == "Thinking level selected: low"
    }));
}

/// Verifies durable selection history reloads with the fallback provenance.
///
/// - `service`: Application service owning the session state.
/// - `context`: Authorized context for the session read.
/// - `session`: Session identity whose history is reloaded.
/// - `selected`: Selection returned by the latest successful mutation.
fn assert_replayed_selection_history(
    service: &ApplicationService<Fixture>,
    context: &ApplicationContext,
    session: EntityId,
    selected: &scriberig_session::SessionExecutionSelection,
) {
    let reloaded = service
        .read_session(context, &SessionRequest { session })
        .expect("selection history reload");
    assert_eq!(reloaded.execution.history, selected.history);
    assert!(reloaded.execution.history.iter().any(|change| {
        change.kind == scriberig_session::AgentActivityKind::ThinkingLevelChanged
            && change.thinking_level.as_deref() == Some("low")
    }));
    assert!(reloaded.execution.history.iter().any(|change| {
        change.revision == 3
            && change.kind == scriberig_session::AgentActivityKind::ModelSelectionChanged
            && change.model_id.as_deref() == Some("model-b")
    }));
    assert!(reloaded.execution.history.iter().any(|change| {
        change.revision == 3
            && change.kind == scriberig_session::AgentActivityKind::ThinkingLevelChanged
            && change.thinking_level.as_deref() == Some("low")
    }));
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

#[path = "session_runtime_parts/part1.rs"]
mod part1;
