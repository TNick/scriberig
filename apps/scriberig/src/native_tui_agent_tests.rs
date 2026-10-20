//! Verifies exact project scope survives native agent continuation.

use std::sync::Arc;

use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelResponse, ProviderMetadata,
};
use scriberig_agent_runtime::{AgentMessage, ModelProvider};
use scriberig_app::{
    AgentCommandRequest, ApplicationBackendComposition, ApplicationContext, ApplicationService,
    CapabilityContext, CapabilityId, PlanCreateRequest, PlanLifecycleAction, PlanLifecycleRequest,
    PlanNodeDraft, SessionBackend, SessionExecutionSelection, StoreInspectionBackend,
    StoreOperationProvider,
};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_session::{
    AgentOutput, Branch, ContinuationAnchor, ContinuationOrigin, ContinuationProvenance,
    HumanInput, ModelAssignment, Session, SessionStore, Turn,
};
use scriberig_store_sqlite::SqliteStore;
use scriberig_tui::{
    action::Action,
    effect::Effect,
    service::{ApplicationOperationAdapter, execute_effect},
    state::{TuiState, reduce},
};

use super::{
    NativeProvider, NativeTuiAgentRunner, model_catalog_from_models, previous_workspace_selection,
};
use crate::native_tui_agent::model_selection::resolve_selected_runtime_model;

/// Builds the upstream catalog used by native fake-provider fixtures.
///
/// # Returns
/// A local provider model matching the fake runtime fallback.
fn native_test_upstream_models() -> Vec<rpi_ai::Model> {
    plan::upstream_model_catalog("local", &["fake-model"])
}

/// Runs the first typed message through the composed TUI and fake provider.
#[test]
#[allow(clippy::too_many_lines)] // Covers the complete shared first-message flow.
fn first_conversation_message_runs_and_persists_fake_provider_answer() {
    let root = std::env::temp_dir().join(format!("scriberig-first-message-{0}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("fixture root");
    let project_path = root.join("project.sqlite");
    let session_path = root.join("session.sqlite");
    let project_store = Arc::new(SqliteStore::open(&project_path).expect("project store"));
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&project_store)),
        StoreOperationProvider::new(Arc::clone(&project_store)),
    );
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(&session_path).expect("session store"),
    );
    let session_repository: Arc<dyn scriberig_session::SessionStore> = session_store.clone();
    let service = ApplicationService::new(backend)
        .with_session_store(session_repository)
        .with_upstream_models(native_test_upstream_models());
    let provider =
        scriberig_agent_runtime::provider::FakeProvider::from_responses([Ok(ModelResponse {
            text: Some("Welcome to the project.".to_owned()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            usage: None,
            metadata: ProviderMetadata::default(),
        })]);
    let runner = NativeTuiAgentRunner {
        providers: std::collections::BTreeMap::from([(
            "local".to_owned(),
            NativeProvider::Fake(provider.clone()),
        )]),
        model: "fake-model".to_owned(),
        model_catalog: model_catalog_from_models(native_test_upstream_models()),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
    };
    let adapter = ApplicationOperationAdapter::new(service).with_agent_runner(runner);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_start").expect("agent grant"),
            CapabilityId::new("agent_continue").expect("continuation grant"),
            CapabilityId::new("agent_plan").expect("session mutation grant"),
            CapabilityId::new("inspect.entity").expect("inspect grant"),
            CapabilityId::new("session_branch_tree").expect("session grant"),
        ]),
        ..ApplicationContext::empty()
    };
    let mut state = TuiState::default();
    state.conversation_composer.draft = "Onboard the current project.".to_owned();
    let effects = reduce(&mut state, Action::InputSubmitted);
    let prompt = match effects.as_slice() {
        [Effect::StartNewConversation { prompt, .. }] => prompt.clone(),
        _ => panic!("first message should use shared session start"),
    };

    let accepted = execute_effect(
        &adapter,
        &context,
        Effect::StartNewConversation {
            prompt,
            selection: None,
        },
    );
    assert!(
        matches!(accepted, Action::ConversationAgentAccepted(_)),
        "new conversation effect failed: {accepted:?}"
    );
    let refresh_effects = reduce(&mut state, accepted);
    assert_eq!(state.conversation_composer.draft, "");
    let (request_id, session) = match refresh_effects.as_slice() {
        [
            Effect::ReadSession {
                request_id,
                session,
                branch: None,
            },
        ] => (*request_id, *session),
        _ => panic!("accepted first message should request durable refresh: {refresh_effects:?}"),
    };
    let refreshed = execute_effect(
        &adapter,
        &context,
        Effect::ReadSession {
            request_id,
            session,
            branch: None,
        },
    );
    reduce(&mut state, refreshed);

    let durable = session_store
        .load(session)
        .expect("load persisted session")
        .expect("new session exists");
    let root_branch = durable.branches.first().expect("root branch");
    assert_eq!(root_branch.turns.len(), 1);
    assert_eq!(
        root_branch.turns[0].human.as_ref().unwrap().text,
        "Onboard the current project."
    );
    assert_eq!(
        root_branch.turns[0].agent.as_ref().unwrap().text,
        "Welcome to the project."
    );
    assert!(state.conversation.is_some());
    assert_eq!(provider.requests().len(), 1);
    drop(adapter);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

#[path = "native_tui_agent_tests/decomposition.rs"]
mod decomposition;
#[path = "native_tui_agent_tests/failed_message.rs"]
mod failed_message;
#[path = "native_tui_agent_tests_part1.rs"]
mod part1;
#[path = "native_tui_agent_plan_tests.rs"]
mod plan;
