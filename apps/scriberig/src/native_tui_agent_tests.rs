//! Verifies exact project scope survives native agent continuation.

use std::sync::Arc;

use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, ModelResponse, ProviderMetadata, ToolCall,
};
use scriberig_agent_runtime::{AgentMessage, ModelProvider};
use scriberig_app::{
    AgentCommandRequest, ApplicationBackendComposition, ApplicationContext, ApplicationService,
    CapabilityContext, CapabilityId, ModelLibrary, PlanCreateRequest, PlanLifecycleAction,
    PlanLifecycleRequest, PlanNodeDraft, SessionBackend, SessionExecutionSelection,
    StoreInspectionBackend, StoreOperationProvider,
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

use super::{NativeProvider, NativeTuiAgentRunner, previous_workspace_selection};
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
        model_library: ModelLibrary::default(),
        upstream_models: native_test_upstream_models(),
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

/// Persists autonomous sibling decomposition through the product runtime.
#[test]
#[allow(clippy::too_many_lines)] // The scenario proves recursive branch results end to end.
fn autonomous_decomposition_persists_children_and_compact_parent_results() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-autonomous-decomposition-{}",
        EntityId::new()
    ));
    std::fs::create_dir_all(&root).expect("fixture root");
    let project_store = Arc::new(SqliteStore::open(root.join("project.sqlite")).expect("project"));
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&project_store)),
        StoreOperationProvider::new(Arc::clone(&project_store)),
    );
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(root.join("session.sqlite"))
            .expect("session store"),
    );
    let session_repository: Arc<dyn scriberig_session::SessionStore> = session_store.clone();
    let service = ApplicationService::new(backend)
        .with_session_store(session_repository)
        .with_upstream_models(native_test_upstream_models());
    let provider = FakeProvider::from_responses([
        Ok(delegation_response("architecture", "architecture")),
        Ok(final_response("ARCHITECTURE_RESULT")),
        Ok(delegation_response("implementation", "implementation")),
        Ok(final_response("IMPLEMENTATION_RESULT")),
        Ok(final_response("decomposition complete")),
    ]);
    let runner = NativeTuiAgentRunner {
        providers: std::collections::BTreeMap::from([(
            "local".to_owned(),
            NativeProvider::Fake(provider.clone()),
        )]),
        model: "fake-model".to_owned(),
        model_library: ModelLibrary::default(),
        upstream_models: native_test_upstream_models(),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
    };
    let adapter = ApplicationOperationAdapter::new(service).with_agent_runner(runner);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_start").expect("agent grant"),
            CapabilityId::new("agent_continue").expect("continuation grant"),
            CapabilityId::new("agent_plan").expect("plan grant"),
            CapabilityId::new("agent.delegate").expect("delegation grant"),
            CapabilityId::new("inspect.entity").expect("inspection grant"),
            CapabilityId::new("session_branch_tree").expect("session grant"),
        ]),
        ..ApplicationContext::empty()
    };
    let mut state = TuiState::default();
    state.conversation_composer.draft = "Split the work into two investigations.".to_owned();
    let effects = reduce(&mut state, Action::InputSubmitted);
    let [
        Effect::StartNewConversation {
            prompt,
            selection: None,
        },
    ] = effects.as_slice()
    else {
        panic!("first message should create a shared session");
    };
    let accepted = execute_effect(
        &adapter,
        &context,
        Effect::StartNewConversation {
            prompt: prompt.clone(),
            selection: None,
        },
    );
    let refresh_effects = reduce(&mut state, accepted);
    let [
        Effect::ReadSession {
            request_id,
            session,
            branch: None,
        },
    ] = refresh_effects.as_slice()
    else {
        panic!("accepted work should refresh the durable session");
    };
    let refreshed = execute_effect(
        &adapter,
        &context,
        Effect::ReadSession {
            request_id: *request_id,
            session: *session,
            branch: None,
        },
    );
    reduce(&mut state, refreshed);
    let durable = session_store
        .load(*session)
        .expect("load session")
        .expect("session exists");
    assert_eq!(durable.branches.len(), 3);
    assert_eq!(durable.branches[0].children.len(), 2);
    assert!(durable.branches[1..].iter().all(|branch| {
        branch.status == scriberig_session::BranchStatus::Completed
            && branch.result.is_some()
            && branch.turns.iter().any(|turn| turn.human.is_some())
    }));
    assert_eq!(
        durable.branches[0]
            .tool_invocations
            .iter()
            .filter(|tool| tool.name == "delegate")
            .count(),
        2
    );
    let requests = provider.requests();
    assert_eq!(requests.len(), 5);
    assert!(
        requests[1]
            .messages
            .iter()
            .any(|message| { message.content.contains("architecture") })
    );
    assert!(
        requests[2]
            .messages
            .iter()
            .any(|message| { message.content.contains("ARCHITECTURE_RESULT") })
    );
    assert!(
        requests[2]
            .messages
            .iter()
            .all(|message| { !message.content.contains("architecture private prompt") })
    );

    drop(adapter);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

/// Creates a deterministic provider response that delegates one child branch.
/// # Parameters
/// - `id`: Input for this operation.
/// - `purpose`: Input for this operation.
/// # Returns
/// The operation result.
fn delegation_response(id: &str, purpose: &str) -> ModelResponse {
    ModelResponse {
        text: None,
        tool_calls: vec![ToolCall {
            id: format!("delegate-{id}"),
            name: "delegate".to_owned(),
            arguments: serde_json::json!({
                "purpose": purpose,
                "prompt": format!("{purpose} private prompt"),
                "capabilities": ["inspect.entity"],
                "expected_result": format!("compact {purpose} result")
            }),
        }],
        finish_reason: FinishReason::ToolCall,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}

/// Creates a deterministic final provider response.
/// # Parameters
/// - `text`: Input for this operation.
/// # Returns
/// The operation result.
fn final_response(text: &str) -> ModelResponse {
    ModelResponse {
        text: Some(text.to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}

#[path = "native_tui_agent_tests/failed_message.rs"]
mod failed_message;
#[path = "native_tui_agent_tests_part1.rs"]
mod part1;
#[path = "native_tui_agent_plan_tests.rs"]
mod plan;
