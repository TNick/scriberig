//! Verifies exact project scope survives native agent continuation.

use std::sync::Arc;

use rpi_ai::providers::faux::FauxScript;
use scriberig_agent_runtime::continuation::EphemeralContinuationOwner;
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

use super::{NativeScenario, NativeTuiAgentRunner, previous_workspace_selection};
use crate::native_tui_agent::model_selection::resolve_selected_runtime_model;

/// Builds the upstream catalog used by native fake-provider fixtures.
///
/// # Returns
/// A local provider model matching the fake runtime fallback.
fn native_test_upstream_models() -> Vec<rpi_ai::Model> {
    plan::upstream_model_catalog("local", &["fake-model"])
}

/// Builds a shared upstream model catalog for deterministic provider fixtures.
///
/// # Parameters
/// - `models`: Provider-qualified models retained by the fixture.
///
/// # Returns
/// A live model catalog containing every supplied provider row.
pub(super) fn model_catalog_from_models(
    models: Vec<rpi_ai::Model>,
) -> rpi_model_runtime::ModelCatalog {
    let catalog = rpi_model_runtime::ModelCatalog::new();
    catalog.reconcile_models(models);
    catalog
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
    let provider = FauxScript::new().with_text("Welcome to the project.");
    let runner = NativeTuiAgentRunner {
        providers: std::collections::BTreeMap::from([(
            "local".to_owned(),
            NativeScenario::Faux(provider.clone()),
        )]),
        model: "fake-model".to_owned(),
        upstream_credentials: None,
        upstream_enabled: false,
        model_catalog: model_catalog_from_models(native_test_upstream_models()).clone(),
        provider_extension_runtime: scriberig_app::ProviderExtensionRuntime::with_model_catalog(
            model_catalog_from_models(native_test_upstream_models()).clone(),
        ),
        plugin_command_worker: std::sync::Mutex::new(None),
        extension_component_mailbox: rpi_extensions::ExtensionStatusMailbox::new(),
        provider_lifecycle:
            crate::native_tui_agent::provider_readiness::ProviderLifecycleState::new(),
        model_config: std::sync::Mutex::new((rpi_model_runtime::ModelsConfig::default(), None)),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
        continuations: EphemeralContinuationOwner::default(),
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
    assert_eq!(root_branch.model_invocations.len(), 1);
    assert!(root_branch.model_invocations[0].request.is_some());
    drop(adapter);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

#[path = "native_tui_agent_tests/continuation_divergence.rs"]
mod continuation_divergence;
#[path = "native_tui_agent_tests/continuation_fixture.rs"]
mod continuation_fixture;
#[path = "native_tui_agent_tests/continuation_lifecycle.rs"]
mod continuation_lifecycle;
#[path = "native_tui_agent_tests/decomposition.rs"]
mod decomposition;
#[path = "native_tui_agent_tests/failed_message.rs"]
mod failed_message;
#[path = "native_tui_agent_tests/model_property_request.rs"]
mod model_property_request;
#[path = "native_tui_agent_tests_part1.rs"]
mod part1;
#[path = "native_tui_agent_plan_tests.rs"]
mod plan;
#[path = "native_tui_agent_tests/readiness_generation.rs"]
mod readiness_generation;
#[path = "native_tui_agent_tests/upstream_activity.rs"]
mod upstream_activity;
#[path = "native_tui_agent_tests/upstream_budgets.rs"]
mod upstream_budgets;
#[path = "native_tui_agent_tests/upstream_cancellation.rs"]
mod upstream_cancellation;
#[path = "native_tui_agent_tests/upstream_fixture.rs"]
mod upstream_fixture;
#[path = "native_tui_agent_tests/upstream_surface_parity.rs"]
mod upstream_surface_parity;
