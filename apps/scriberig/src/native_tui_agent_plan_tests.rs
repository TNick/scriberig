//! Verifies production plan execution through the native TUI runtime.

use super::*;
use crate::native_tui_agent::tests::model_catalog_from_models;

#[path = "native_tui_agent_plan_tests/support.rs"]
mod support;
use support::agentic_fixture_root;

/// Builds provider models matching the plan test's safe library projection.
///
/// - `provider`: Upstream provider identity.
/// - `model_ids`: Provider-native model ids and display names.
///
/// A catalog used by the plan selection runtime.
/// # Returns
/// The operation result.
pub(super) fn upstream_model_catalog(provider: &str, model_ids: &[&str]) -> Vec<rpi_ai::Model> {
    model_ids
        .iter()
        .map(|model_id| {
            rpi_ai::Model::new(
                *model_id,
                *model_id,
                rpi_ai::Api::AnthropicMessages,
                provider,
                "https://example.test",
            )
        })
        .collect()
}

/// Builds a successful fake-provider response with the supplied text.
///
/// - `text`: Final response content.
///
/// # Returns
/// A completed non-tool provider response.
///
/// Starts a durable plan through the native lifecycle and executes its worker.
#[test]
#[allow(clippy::too_many_lines)] // Keeps worker assignment and resume evidence end to end.
fn plan_start_runs_claimed_branch_with_the_fake_provider() {
    let root = agentic_fixture_root("native-plan");
    let project_store = Arc::new(SqliteStore::open(root.join("project.sqlite")).expect("project"));
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&project_store)),
        StoreOperationProvider::new(Arc::clone(&project_store)),
    );
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(root.join("session.sqlite"))
            .expect("session store"),
    );
    let session_repository: Arc<dyn SessionStore> = session_store.clone();
    let upstream_models =
        upstream_model_catalog("fake", &["wire-plan-model-a", "wire-plan-model-b"]);
    let service = ApplicationService::new(backend)
        .with_session_store(session_repository)
        .with_upstream_models(upstream_models.clone());
    let session_id = EntityId::new();
    let root_branch = EntityId::new();
    let mut branch = Branch::new(root_branch, "root").expect("root branch");
    branch.status = scriberig_session::BranchStatus::Completed;
    let context = ApplicationContext {
        capabilities: CapabilityContext::new([
            CapabilityId::new("agent_plan").expect("plan"),
            CapabilityId::new("session_branch_tree").expect("session tree"),
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
    .expect("save root session");
    let plan = SessionBackend::create_plan(
        &service,
        &context,
        &PlanCreateRequest {
            session: session_id,
            anchor: ContinuationAnchor::BranchHead {
                branch: root_branch,
            },
            goal: "inspect parser behavior".to_owned(),
            nodes: vec![
                PlanNodeDraft {
                    key: "inspect".to_owned(),
                    parent_index: None,
                    title: "Inspect parser".to_owned(),
                    purpose: "Summarize the existing parser behavior.".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: Some(ModelAssignment {
                        provider_id: "plan-provider-a".to_owned(),
                        model_id: "plan-model-a".to_owned(),
                        thinking_level: Some("high".to_owned()),
                    }),
                },
                PlanNodeDraft {
                    key: "parallel-inspect".to_owned(),
                    parent_index: None,
                    title: "Inspect parser fixtures".to_owned(),
                    purpose: "Summarize parser fixture behavior.".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: Some(ModelAssignment {
                        provider_id: "plan-provider-b".to_owned(),
                        model_id: "plan-model-b".to_owned(),
                        thinking_level: Some("high".to_owned()),
                    }),
                },
                PlanNodeDraft {
                    key: "dependent-check".to_owned(),
                    parent_index: None,
                    title: "Check parser summary".to_owned(),
                    purpose: "Validate the parser summary against fixtures.".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: Some(ModelAssignment {
                        provider_id: "plan-provider-a".to_owned(),
                        model_id: "plan-model-a".to_owned(),
                        thinking_level: Some("high".to_owned()),
                    }),
                },
            ],
            dependencies: vec![(0, 2)],
        },
    )
    .expect("create durable plan");
    let paused_plan = SessionBackend::create_plan(
        &service,
        &context,
        &PlanCreateRequest {
            session: session_id,
            anchor: ContinuationAnchor::BranchHead {
                branch: root_branch,
            },
            goal: "validate parser behavior".to_owned(),
            nodes: vec![PlanNodeDraft {
                key: "validate".to_owned(),
                parent_index: None,
                title: "Validate parser".to_owned(),
                purpose: "Report whether parser fixtures validate.".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: Some(ModelAssignment {
                    provider_id: "plan-provider-a".to_owned(),
                    model_id: "plan-model-a".to_owned(),
                    thinking_level: Some("high".to_owned()),
                }),
            }],
            dependencies: Vec::new(),
        },
    )
    .expect("create second plan");
    let started = SessionBackend::plan_lifecycle(
        &service,
        &context,
        &PlanLifecycleRequest {
            session: session_id,
            plan: paused_plan.id,
            expected_revision: paused_plan.revision,
        },
        PlanLifecycleAction::Start,
    )
    .expect("claim second plan");
    let paused = SessionBackend::plan_lifecycle(
        &service,
        &context,
        &PlanLifecycleRequest {
            session: session_id,
            plan: started.id,
            expected_revision: started.revision,
        },
        PlanLifecycleAction::Pause,
    )
    .expect("pause second plan");
    let provider_a = rpi_ai::providers::faux::FauxScript::new()
        .with_text("Independent result.")
        .with_text("Parser validation completed.")
        .with_text("Resumed parser validation completed.");
    let provider_b = rpi_ai::providers::faux::FauxScript::new().with_text("Independent result.");
    let runner = NativeTuiAgentRunner {
        providers: std::collections::BTreeMap::from([
            (
                "fake-a".to_owned(),
                NativeScenario::Faux(provider_a.clone()),
            ),
            (
                "fake-b".to_owned(),
                NativeScenario::Faux(provider_b.clone()),
            ),
        ]),
        model: "fake-model".to_owned(),
        upstream_credentials: None,
        upstream_enabled: false,
        model_catalog: model_catalog_from_models(upstream_models),
        model_config: std::sync::Mutex::new((rpi_model_runtime::ModelsConfig::default(), None)),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
        continuations: std::sync::Mutex::default(),
    };
    let adapter = ApplicationOperationAdapter::new(service).with_agent_runner(runner);
    let session_snapshot = session_store
        .load(session_id)
        .expect("load plan before start")
        .expect("session exists");
    let mut tui_state = TuiState::default();
    tui_state.session_snapshot = Some(session_snapshot);
    tui_state.conversation_cursor = Some(
        scriberig_tui::view::conversation::ConversationSource::Plan(plan.id),
    );
    tui_state.conversation_composer = scriberig_tui::state::ConversationComposerState::new(
        scriberig_tui::state::ConversationTarget::Existing {
            session: session_id,
            reply_anchor: ContinuationAnchor::BranchHead {
                branch: root_branch,
            },
        },
        "/plan start".to_owned(),
    );
    let effects = reduce(&mut tui_state, Action::InputSubmitted);
    let [start_effect] = effects.as_slice() else {
        panic!("/plan start should produce one lifecycle effect: {effects:?}");
    };
    assert!(
        matches!(
            start_effect,
            Effect::PlanLifecycle { request, action: PlanLifecycleAction::Start }
                if request.session == session_id && request.plan == plan.id
        ),
        "unexpected /plan start routing: {effects:?}"
    );
    let action = execute_effect(&adapter, &context, start_effect.clone());
    let Action::PlanUpdated(updated) = action else {
        panic!("plan start should complete its ready worker: {action:?}");
    };
    assert_eq!(updated.status, scriberig_session::PlanStatus::Completed);
    assert!(
        updated
            .nodes
            .iter()
            .all(|node| node.status == scriberig_session::PlanNodeStatus::Completed)
    );
    let durable = session_store
        .load(session_id)
        .expect("load session")
        .expect("session exists");
    let invocations = durable
        .branches
        .iter()
        .flat_map(|branch| &branch.model_invocations)
        .collect::<Vec<_>>();
    let requests_a = invocations
        .iter()
        .filter(|invocation| invocation.provider_id.as_deref() == Some("fake-a"))
        .filter_map(|invocation| invocation.request.as_ref())
        .collect::<Vec<_>>();
    let requests_b = invocations
        .iter()
        .filter(|invocation| invocation.provider_id.as_deref() == Some("fake-b"))
        .filter_map(|invocation| invocation.request.as_ref())
        .collect::<Vec<_>>();
    assert_eq!(requests_a.len(), 2);
    assert_eq!(requests_b.len(), 1);
    assert!(requests_a.iter().any(|request| {
        request.model == "wire-plan-model-a" && request.thinking_level.as_deref() == Some("high")
    }));
    assert!(requests_b.iter().any(|request| {
        request.model == "wire-plan-model-b" && request.thinking_level.as_deref() == Some("high")
    }));
    assert!(
        requests_a[1]
            .messages
            .iter()
            .any(|message| message.content.contains("Independent result."))
    );
    assert!(requests_a[1].messages.iter().all(|message| {
        !message
            .content
            .contains("Summarize the existing parser behavior.")
    }));
    let worker = durable
        .branches
        .iter()
        .find(|candidate| candidate.result.is_some())
        .expect("durable worker result");
    assert_eq!(
        worker.result.as_ref().unwrap().summary.as_deref(),
        Some("Independent result.")
    );
    let first_branch = updated.nodes[0].branch.expect("first worker branch");
    let second_branch = updated.nodes[1].branch.expect("second worker branch");
    assert_ne!(first_branch, second_branch);
    let first_result = durable
        .branches
        .iter()
        .find(|branch| branch.id == first_branch)
        .and_then(|branch| branch.result.as_ref())
        .expect("first worker result");
    let second_result = durable
        .branches
        .iter()
        .find(|branch| branch.id == second_branch)
        .and_then(|branch| branch.result.as_ref())
        .expect("second worker result");
    assert!(
        first_result
            .result_handles
            .contains(&"model-profile:plan-model-a-profile".to_owned())
    );
    assert!(
        first_result
            .result_handles
            .contains(&"provider-profile:fake-a".to_owned())
    );
    assert!(
        second_result
            .result_handles
            .contains(&"model-profile:plan-model-b-profile".to_owned())
    );
    assert!(
        second_result
            .result_handles
            .contains(&"provider-profile:fake-b".to_owned())
    );

    let resumed = execute_effect(
        &adapter,
        &context,
        Effect::PlanLifecycle {
            request: PlanLifecycleRequest {
                session: session_id,
                plan: paused.id,
                expected_revision: paused.revision,
            },
            action: PlanLifecycleAction::Resume,
        },
    );
    let Action::PlanUpdated(resumed) = resumed else {
        panic!("plan resume should execute its ready worker: {resumed:?}");
    };
    assert_eq!(resumed.status, scriberig_session::PlanStatus::Completed);
    let resumed_session = session_store
        .load(session_id)
        .expect("load resumed plan session")
        .expect("resumed session exists");
    assert_eq!(
        resumed_session
            .branches
            .iter()
            .flat_map(|branch| &branch.model_invocations)
            .filter(|invocation| invocation.provider_id.as_deref() == Some("fake-a"))
            .count(),
        3
    );

    drop(adapter);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

#[path = "native_tui_agent_plan_tests/http.rs"]
mod http;
#[path = "native_tui_agent_plan_tests/integration.rs"]
mod integration;
