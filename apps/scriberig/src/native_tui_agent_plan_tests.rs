//! Verifies production plan execution through the native TUI runtime.

use super::*;

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
fn final_response(text: &str) -> ModelResponse {
    ModelResponse {
        text: Some(text.to_owned()),
        tool_calls: Vec::new(),
        finish_reason: FinishReason::Stop,
        usage: None,
        metadata: ProviderMetadata::default(),
    }
}

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
    let provider_a = FakeProvider::from_responses([
        Ok(final_response("Independent result.")),
        Ok(final_response("Parser validation completed.")),
        Ok(final_response("Resumed parser validation completed.")),
    ]);
    let provider_b = FakeProvider::from_responses([Ok(final_response("Independent result."))]);
    let runner = NativeTuiAgentRunner {
        providers: std::collections::BTreeMap::from([
            (
                "fake-a".to_owned(),
                NativeProvider::Fake(provider_a.clone()),
            ),
            (
                "fake-b".to_owned(),
                NativeProvider::Fake(provider_b.clone()),
            ),
        ]),
        model: "fake-model".to_owned(),
        model_catalog: model_catalog_from_models(upstream_models),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
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
    assert_eq!(provider_a.requests().len(), 2);
    assert_eq!(provider_b.requests().len(), 1);
    assert!(provider_a.requests().iter().any(|request| {
        request.model == "wire-plan-model-a" && request.thinking_level.as_deref() == Some("high")
    }));
    assert!(provider_b.requests().iter().any(|request| {
        request.model == "wire-plan-model-b" && request.thinking_level.as_deref() == Some("high")
    }));
    assert!(
        provider_a.requests()[1]
            .messages
            .iter()
            .any(|message| { message.content.contains("Independent result.") })
    );
    assert!(provider_a.requests()[1].messages.iter().all(|message| {
        !message
            .content
            .contains("Summarize the existing parser behavior.")
    }));
    let durable = session_store
        .load(session_id)
        .expect("load session")
        .expect("session exists");
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
    assert_eq!(provider_a.requests().len(), 3);

    drop(adapter);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

#[path = "native_tui_agent_plan_tests/http.rs"]
mod http;
#[path = "native_tui_agent_plan_tests/integration.rs"]
mod integration;

/// Creates and returns a uniquely named agent-owned test directory.
///
/// - `label`: Short representative name for the temporary fixture.
///
/// A path below `playground/agentic-work` with the required timestamp prefix.
/// # Returns
/// The created fixture directory.
fn agentic_fixture_root(label: &str) -> std::path::PathBuf {
    assert!(
        label.len() <= 40,
        "fixture label must be at most 40 characters"
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time after Unix epoch");
    let seconds = i64::try_from(now.as_secs()).expect("timestamp fits i64");
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    }
    .div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524
        - day_of_era / 146_096)
        .div_euclid(365);
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2).div_euclid(153);
    let day = day_of_year - (153 * month_part + 2).div_euclid(5) + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let hour = day_seconds / 3_600;
    let minute = day_seconds % 3_600 / 60;
    let second = day_seconds % 60;
    let timestamp = format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}");
    let identifier = EntityId::new().to_string();
    let suffix = &identifier[4..12];
    let directory_name = format!("{timestamp} {label}-{suffix}");
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../playground/agentic-work")
        .join(directory_name);
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}
