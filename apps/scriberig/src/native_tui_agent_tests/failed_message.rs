//! Verifies failed first turns remain visible in the durable conversation.

use super::model_catalog_from_models;
use super::*;
use rpi_ai::providers::faux::{FauxScript, FauxStep};
use scriberig_agent_runtime::continuation::EphemeralContinuationOwner;

#[expect(
    clippy::too_many_lines,
    reason = "The integration scenario asserts the durable error, tree projection, and operation evidence together."
)]
/// Preserves an accepted message and presents a deterministic provider failure.
#[test]
fn first_conversation_failure_remains_in_tree_with_error() {
    let root = std::env::temp_dir().join(format!("scriberig-failed-message-{}", EntityId::new()));
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
    let model = native_test_upstream_models()[0].clone();
    let provider = FauxScript::new();
    provider.set_responses(vec![FauxStep::factory(move |_| {
        crate::scenario_helpers::error_response(&model, "deterministic provider failure".to_owned())
    })]);
    let runner = NativeTuiAgentRunner {
        providers: std::collections::BTreeMap::from([(
            "local".to_owned(),
            NativeScenario::Faux(provider),
        )]),
        model: "fake-model".to_owned(),
        upstream_credentials: None,
        upstream_enabled: false,
        model_catalog: model_catalog_from_models(native_test_upstream_models()),
        model_config: std::sync::Mutex::new((rpi_model_runtime::ModelsConfig::default(), None)),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
        continuations: EphemeralContinuationOwner::default(),
    };
    let adapter = ApplicationOperationAdapter::new(service).with_agent_runner(runner);
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_start").expect("agent grant"),
            CapabilityId::new("inspect.entity").expect("inspection grant"),
            CapabilityId::new("session_branch_tree").expect("session grant"),
        ]),
        ..ApplicationContext::empty()
    };
    let mut state = TuiState::default();
    state.conversation_composer.draft = "1+1".to_owned();
    let submit = reduce(&mut state, Action::InputSubmitted);
    let [Effect::StartNewConversation { prompt, .. }] = submit.as_slice() else {
        panic!("first message should use the shared session start");
    };
    let action = execute_effect(
        &adapter,
        &context,
        Effect::StartNewConversation {
            prompt: prompt.clone(),
            selection: None,
        },
    );
    let accepted = reduce(&mut state, action);
    let [
        Effect::ReadSession {
            request_id,
            session,
            ..
        },
    ] = accepted.as_slice()
    else {
        panic!("accepted failure should refresh its durable conversation");
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

    let conversation = state.conversation.as_ref().expect("loaded conversation");
    assert!(conversation.rows.iter().any(|row| row.label == "You: 1+1"));
    assert!(
        conversation
            .rows
            .iter()
            .any(|row| row.label.contains("deterministic provider failure")),
        "conversation rows: {:?}",
        conversation.rows
    );
    assert_eq!(state.conversation_composer.draft, "");
    assert!(
        state
            .conversation_composer
            .error
            .as_deref()
            .is_some_and(|message| message.contains("deterministic provider failure"))
    );

    let durable = session_store
        .load(*session)
        .expect("load failed session")
        .expect("failed session exists");
    let failed_root = durable
        .branches
        .iter()
        .find(|branch| branch.parent.is_none())
        .expect("root branch exists");
    let failed_invocation = failed_root
        .model_invocations
        .last()
        .expect("provider attempt is durable");
    let request = failed_invocation
        .request
        .as_ref()
        .expect("prepared upstream request is retained");
    assert!(
        request
            .messages
            .iter()
            .any(|message| { message.role == "user" && message.content.contains("1+1") })
    );

    drop(adapter);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}
