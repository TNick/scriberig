//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies historical tui continuation compiles exact provider context.
#[test]
#[allow(clippy::too_many_lines)] // The scenario checks exact history, ancestry, and exclusions.
fn historical_tui_continuation_compiles_exact_provider_context() {
    let root = std::env::temp_dir().join(format!("scriberig-reply-{0}", EntityId::new()));
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
    let session_id = EntityId::from_bytes([81; 16]);
    let root_id = EntityId::from_bytes([82; 16]);
    let turn_ids = [
        EntityId::from_bytes([83; 16]),
        EntityId::from_bytes([84; 16]),
        EntityId::from_bytes([85; 16]),
    ];
    let mut branch = Branch::new(root_id, "root").expect("root branch");
    for (index, turn_id) in turn_ids.into_iter().enumerate() {
        branch.turns.push(Turn {
            id: turn_id,
            ordinal: u64::try_from(index + 1).expect("ordinal"),
            human: Some(HumanInput {
                text: format!("turn {} user", index + 1),
            }),
            agent: Some(AgentOutput {
                text: format!("turn {} answer", index + 1),
            }),
            tool_invocation: None,
            tool_result: None,
        });
    }
    let provider =
        scriberig_agent_runtime::provider::FakeProvider::from_responses([Ok(ModelResponse {
            text: Some("historical answer".to_owned()),
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
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_start").expect("agent grant"),
            CapabilityId::new("agent_continue").expect("continuation grant"),
            CapabilityId::new("agent_plan").expect("session mutation grant"),
            CapabilityId::new("inspect.entity").expect("inspect grant"),
            CapabilityId::new("session_branch_tree").expect("session read grant"),
        ]),
        ..ApplicationContext::empty()
    };
    SessionBackend::save_session(
        &service,
        &context,
        &Session {
            id: session_id,
            branches: vec![branch],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: SessionExecutionSelection::default(),
        },
    )
    .expect("save conversation");
    let adapter = ApplicationOperationAdapter::new(service).with_agent_runner(runner);

    let accepted = execute_effect(
        &adapter,
        &context,
        Effect::ContinueConversation {
            session: session_id,
            anchor: ContinuationAnchor::AssistantOutput {
                branch: root_id,
                turn: turn_ids[1],
            },
            prompt: "continue from turn two".to_owned(),
        },
    );
    let Action::ConversationAgentAccepted(handle) = accepted else {
        panic!("historical continuation should run through the native runner: {accepted:?}");
    };
    let durable = session_store
        .load(session_id)
        .expect("reload session")
        .expect("session exists");
    let context_message = &provider.requests()[0].messages[0].content;

    assert!(context_message.contains("turn 1 user"));
    assert!(context_message.contains("turn 2 user"));
    assert!(context_message.contains("turn 2 answer"));
    assert!(!context_message.contains("turn 3 user"));
    assert!(!context_message.contains("turn 3 answer"));
    assert_eq!(context_message.matches("continue from turn two").count(), 1);
    assert_eq!(handle.session, session_id);
    let child = durable
        .branches
        .iter()
        .find(|branch| branch.id == handle.branch.expect("continuation branch"))
        .expect("durable continuation");
    assert_eq!(child.parent, Some(root_id));
    assert_eq!(
        child.continuation.expect("exact provenance").anchor,
        ContinuationAnchor::AssistantOutput {
            branch: root_id,
            turn: turn_ids[1],
        }
    );

    drop(adapter);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

/// Compiles exact historical context for a native continuation without later turns.
#[test]
#[allow(clippy::too_many_lines)] // The context fixture compares exact source and branch history.
fn native_context_uses_exact_anchor_and_current_branch_history() {
    let root = std::env::temp_dir().join(format!("scriberig-context-{0}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("fixture root");
    let database = root.join("state.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let service = ApplicationService::new(backend);
    let session_id = EntityId::from_bytes([31; 16]);
    let root_id = EntityId::from_bytes([32; 16]);
    let child_id = EntityId::from_bytes([33; 16]);
    let anchor_turn = EntityId::from_bytes([34; 16]);
    let later_turn = EntityId::from_bytes([35; 16]);
    let local_turn = EntityId::from_bytes([36; 16]);
    let current_turn = EntityId::from_bytes([37; 16]);
    let mut root_branch = Branch::new(root_id, "root").expect("root");
    root_branch.turns = vec![
        Turn {
            id: anchor_turn,
            ordinal: 1,
            human: Some(HumanInput {
                text: "turn A selected".to_owned(),
            }),
            agent: Some(AgentOutput {
                text: "turn A answer".to_owned(),
            }),
            tool_invocation: None,
            tool_result: None,
        },
        Turn {
            id: later_turn,
            ordinal: 2,
            human: Some(HumanInput {
                text: "turn C excluded".to_owned(),
            }),
            agent: None,
            tool_invocation: None,
            tool_result: None,
        },
    ];
    let mut child =
        Branch::new_child(child_id, root_id, "reply", "continue B").expect("child branch");
    child.continuation = Some(ContinuationProvenance {
        anchor: ContinuationAnchor::AssistantOutput {
            branch: root_id,
            turn: anchor_turn,
        },
        origin: ContinuationOrigin::HumanContinuation,
    });
    child.turns = vec![
        Turn {
            id: local_turn,
            ordinal: 1,
            human: Some(HumanInput {
                text: "child local prior".to_owned(),
            }),
            agent: Some(AgentOutput {
                text: "child local answer".to_owned(),
            }),
            tool_invocation: None,
            tool_result: None,
        },
        Turn {
            id: current_turn,
            ordinal: 2,
            human: Some(HumanInput {
                text: "current prompt".to_owned(),
            }),
            agent: None,
            tool_invocation: None,
            tool_result: None,
        },
    ];
    let mut sibling = Branch::new_child(
        EntityId::from_bytes([38; 16]),
        root_id,
        "sibling",
        "unrelated",
    )
    .expect("sibling");
    sibling.turns.push(Turn {
        id: EntityId::from_bytes([39; 16]),
        ordinal: 1,
        human: Some(HumanInput {
            text: "sibling secret".to_owned(),
        }),
        agent: None,
        tool_invocation: None,
        tool_result: None,
    });
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("session_branch_tree").expect("session read grant"),
            CapabilityId::new("agent_plan").expect("session mutation grant"),
        ]),
        ..ApplicationContext::empty()
    };
    SessionBackend::save_session(
        &service,
        &context,
        &Session {
            id: session_id,
            branches: vec![root_branch, child, sibling],
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: SessionExecutionSelection::default(),
        },
    )
    .expect("persist exact test session");

    let compiled = crate::agent_context::compile_branch_context(
        &service,
        &context,
        session_id,
        child_id,
        "current prompt",
        None,
    )
    .expect("compile native context");

    assert!(compiled.canonical.contains("turn A selected"));
    assert!(compiled.canonical.contains("turn A answer"));
    assert!(compiled.canonical.contains("child local prior"));
    assert!(compiled.canonical.contains("child local answer"));
    assert!(!compiled.canonical.contains("turn C excluded"));
    assert!(!compiled.canonical.contains("sibling secret"));
    assert_eq!(compiled.canonical.matches("current prompt").count(), 1);
    assert!(
        compiled
            .canonical
            .contains("scriberig.context-component.core-invariants@")
    );
    assert!(
        compiled
            .canonical
            .contains("scriberig.context-component.non-root-handoff@")
    );

    let root_compiled = crate::agent_context::compile_branch_context(
        &service,
        &context,
        session_id,
        root_id,
        "root user-facing prompt",
        None,
    )
    .expect("compile root context without project scope");
    assert!(
        root_compiled
            .canonical
            .contains("scriberig.context-component.core-invariants@")
    );
    assert!(
        !root_compiled
            .canonical
            .contains("scriberig.context-component.non-root-handoff@")
    );
    drop(service);
    drop(store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

/// Resolves the selected safe descriptor into provider request settings.
#[test]
fn session_model_and_thinking_selection_resolve_for_native_runtime() {
    let selection = SessionExecutionSelection {
        revision: 2,
        provider_id: Some("openai_compatible".to_owned()),
        model_id: Some("provider-model-2".to_owned()),
        thinking_level: Some("high".to_owned()),
        history: Vec::new(),
    };
    let upstream_model = rpi_ai::Model::new(
        "provider-model-2",
        "Scriberig model",
        rpi_ai::Api::AnthropicMessages,
        "openai_compatible",
        "https://example.test",
    );

    let resolved = resolve_selected_runtime_model(&[upstream_model], &selection, "runtime-default")
        .expect("resolve selected settings");

    assert_eq!(resolved.0, "openai_compatible");
    assert_eq!(resolved.1, "provider-model-2");
    assert_eq!(resolved.2.as_deref(), Some("high"));
    assert_eq!(resolved.3, "rpi1:17:openai_compatible16:provider-model-2");
}

#[path = "native_tui_agent_tests_part1_part2.rs"]
mod part2;
