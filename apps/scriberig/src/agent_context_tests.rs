//! Tests persisted root context compilation for the integrated application.

use std::sync::Arc;

use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    FakeProvider, FinishReason, MessageRole, ModelResponse, ProviderMetadata,
};
use scriberig_agent_runtime::{AgentMessage, ModelProvider};
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, CapabilityContext,
    CapabilityId, SessionBackend, SessionRequest, StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::EntityId;
use scriberig_session::SessionStore;
use scriberig_store_sqlite::SqliteStore;

/// Compiles a persisted parentless root without requiring project scope.
#[test]
fn persisted_root_context_has_core_without_handoff() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = Arc::new(
        SqliteStore::open(temporary.path().join("project.sqlite")).expect("project store"),
    );
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(temporary.path().join("session.sqlite"))
            .expect("session store"),
    );
    let session_repository: Arc<dyn SessionStore> = session_store;
    let service = ApplicationService::new(backend).with_session_store(session_repository);
    let session_id = EntityId::from_bytes([191; 16]);
    let prompt = "inspect the root context path";
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("session tree capability")
        ]),
        ..ApplicationContext::empty()
    };
    let handle = service
        .start_local_agent(&scriberig_app::AgentCommandRequest {
            session: session_id,
            branch: None,
            anchor: None,
            prompt: prompt.to_owned(),
            selection: None,
        })
        .expect("start root agent");

    let session = SessionBackend::read_session(
        &service,
        &context,
        &SessionRequest {
            session: session_id,
        },
    )
    .expect("read persisted session");
    let root = session
        .branches
        .iter()
        .find(|branch| branch.id == handle.root_branch)
        .expect("persisted root branch");
    assert!(root.parent.is_none());

    let compiled = super::compile_branch_context(
        &service,
        &context,
        session_id,
        handle.root_branch,
        prompt,
        None,
    )
    .expect("compile root context");

    assert!(!compiled.canonical.contains("semantic_project_scope"));
    assert!(
        compiled
            .canonical
            .contains("scriberig.context-component.core-invariants@")
    );
    assert!(
        !compiled
            .canonical
            .contains("scriberig.context-component.non-root-handoff@")
    );
    assert_eq!(compiled.canonical.matches(prompt).count(), 1);

    let provider = FakeProvider::from_responses([Ok(ModelResponse {
        text: Some("completed".to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    })]);
    let mut model =
        ProviderModel::new(provider.clone(), "test-model").with_compiled_context(compiled);
    model
        .decide(&[AgentMessage::User(prompt.to_owned())])
        .expect("complete provider request");

    let request = provider.requests().pop().expect("provider-facing request");
    assert!(request.messages[0].content.contains("core-invariant"));
    assert!(!request.messages[0].content.contains("non-root-handoff"));
    assert!(!request.messages[0].content.contains(prompt));
    let user_messages = request
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .collect::<Vec<_>>();
    assert_eq!(user_messages.len(), 1);
    assert_eq!(user_messages[0].content, prompt);
}
