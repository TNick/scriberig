//! Verifies durable autonomous decomposition and compact parent handoff.

use super::model_catalog_from_models;
use super::*;
use rpi_ai::providers::faux::FauxScript;
use scriberig_agent_runtime::continuation::EphemeralContinuationOwner;

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
    let provider = FauxScript::new()
        .with_tool_call(
            "delegate",
            serde_json::json!({
                "purpose": "architecture",
                "prompt": "architecture private prompt",
                "capabilities": ["inspect.entity", "runtime_capabilities"],
                "expected_result": "compact architecture result"
            }),
        )
        .with_tool_call("runtime_capabilities", serde_json::json!({}))
        .with_text("ARCHITECTURE_RESULT")
        .with_tool_call(
            "delegate",
            serde_json::json!({
                "purpose": "implementation",
                "prompt": "implementation private prompt",
                "capabilities": ["inspect.entity", "runtime_capabilities"],
                "expected_result": "compact implementation result"
            }),
        )
        .with_text("IMPLEMENTATION_RESULT")
        .with_text("decomposition complete");
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
            CapabilityId::new("agent_plan").expect("plan grant"),
            CapabilityId::new("agent.delegate").expect("delegation grant"),
            CapabilityId::new("inspect.entity").expect("inspection grant"),
            CapabilityId::new("runtime_capabilities").expect("runtime capabilities grant"),
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
    for (purpose, expected_summary) in [
        ("architecture", "ARCHITECTURE_RESULT"),
        ("implementation", "IMPLEMENTATION_RESULT"),
    ] {
        let child = durable.branches[1..]
            .iter()
            .find(|branch| {
                branch
                    .purpose
                    .as_deref()
                    .is_some_and(|value| value.starts_with(purpose))
            })
            .expect("decomposed child branch");
        let result = child.result.as_ref().expect("persisted child result");
        assert_eq!(result.summary.as_deref(), Some(expected_summary));
        assert!(!result.result_handles.contains(&expected_summary.to_owned()));
    }
    let architecture_result = durable.branches[1..]
        .iter()
        .find(|branch| {
            branch
                .purpose
                .as_deref()
                .is_some_and(|value| value.starts_with("architecture"))
        })
        .and_then(|branch| branch.result.as_ref())
        .expect("architecture child result");
    assert!(
        architecture_result
            .result_handles
            .iter()
            .any(|handle| handle == "runtime_capabilities"),
        "architecture handles: {:?}; child tools: {:?}",
        architecture_result.result_handles,
        durable.branches[1..]
            .iter()
            .map(|branch| (
                branch.purpose.clone(),
                branch
                    .tool_invocations
                    .iter()
                    .map(|tool| tool.name.clone())
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        durable.branches[0]
            .tool_invocations
            .iter()
            .filter(|tool| tool.name == "delegate")
            .count(),
        2
    );
    let parent = &durable.branches[0];
    let child_turn_ids = durable.branches[1..]
        .iter()
        .flat_map(|branch| branch.turns.iter().map(|turn| turn.id))
        .collect::<Vec<_>>();
    let child_model_ids = durable.branches[1..]
        .iter()
        .flat_map(|branch| branch.model_invocations.iter().map(|item| item.id))
        .collect::<Vec<_>>();
    let child_tool_ids = durable.branches[1..]
        .iter()
        .flat_map(|branch| branch.tool_invocations.iter().map(|item| item.id))
        .collect::<Vec<_>>();
    let child_tool_result_ids = durable.branches[1..]
        .iter()
        .flat_map(|branch| branch.tool_results.iter().map(|item| item.id))
        .collect::<Vec<_>>();
    assert_eq!(child_tool_ids.len(), 1);
    assert_eq!(child_tool_result_ids.len(), 1);
    assert!(durable.branches[1..].iter().all(|branch| {
        !branch.model_invocations.is_empty()
            && branch.activity.iter().any(|activity| {
                matches!(
                    activity,
                    scriberig_session::ExecutionActivity::ModelInvocation(id)
                        if branch.model_invocations.iter().any(|item| item.id == *id)
                )
            })
            && branch.activity.iter().any(|activity| {
                matches!(
                    activity,
                    scriberig_session::ExecutionActivity::BranchResult(id)
                        if branch.result.as_ref().is_some_and(|result| result.id == *id)
                )
            })
    }));
    let tool_branch = durable
        .branches
        .iter()
        .find(|branch| {
            branch
                .tool_invocations
                .iter()
                .any(|invocation| child_tool_ids.contains(&invocation.id))
        })
        .expect("child branch with tool evidence");
    assert!(tool_branch.activity.iter().any(|activity| matches!(
        activity,
        scriberig_session::ExecutionActivity::ToolInvocation(id)
            if child_tool_ids.contains(id)
    )));
    assert!(tool_branch.activity.iter().any(|activity| matches!(
        activity,
        scriberig_session::ExecutionActivity::ToolResult(id)
            if child_tool_result_ids.contains(id)
    )));
    assert!(child_model_ids.iter().all(|id| {
        !parent.model_invocations.iter().any(|item| item.id == *id)
            && !parent.activity.iter().any(|activity| {
                matches!(
                    activity,
                    scriberig_session::ExecutionActivity::ModelInvocation(parent_id)
                        if parent_id == id
                )
            })
    }));
    assert!(child_tool_ids.iter().all(|id| {
        !parent.tool_invocations.iter().any(|item| item.id == *id)
            && !parent.activity.iter().any(|activity| {
                matches!(
                    activity,
                    scriberig_session::ExecutionActivity::ToolInvocation(parent_id)
                        if parent_id == id
                )
            })
    }));
    assert!(child_tool_result_ids.iter().all(|id| {
        !parent.tool_results.iter().any(|item| item.id == *id)
            && !parent.activity.iter().any(|activity| {
                matches!(
                    activity,
                    scriberig_session::ExecutionActivity::ToolResult(parent_id)
                        if parent_id == id
                )
            })
    }));
    assert!(child_turn_ids.iter().all(|id| {
        !parent.turns.iter().any(|turn| turn.id == *id)
            && !parent.activity.iter().any(|activity| {
                matches!(
                    activity,
                    scriberig_session::ExecutionActivity::AgentOutput(parent_id)
                        | scriberig_session::ExecutionActivity::HumanInput(parent_id)
                        if parent_id == id
                )
            })
    }));
    assert!(
        parent
            .tool_invocations
            .iter()
            .filter(|tool| tool.name == "delegate")
            .all(|tool| parent.activity.iter().any(|activity| matches!(
                activity,
                scriberig_session::ExecutionActivity::ToolInvocation(id) if *id == tool.id
            )))
    );
    let root_requests = parent
        .model_invocations
        .iter()
        .filter_map(|invocation| invocation.request.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(root_requests.len(), 3);
    assert!(
        root_requests[1]
            .messages
            .iter()
            .any(|message| message.content.contains("ARCHITECTURE_RESULT"))
    );
    assert!(
        root_requests[1]
            .messages
            .iter()
            .all(|message| !message.content.contains("architecture private prompt"))
    );

    drop(adapter);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}
