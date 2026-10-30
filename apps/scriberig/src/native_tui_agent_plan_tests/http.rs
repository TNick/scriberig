//! Exercises plan lifecycle dispatch through the native HTTP protocol contract.

use super::*;
use crate::native_tui_agent::tests::model_catalog_from_models;

type HttpBackend = ApplicationBackendComposition<
    StoreInspectionBackend<Arc<SqliteStore>>,
    StoreOperationProvider<Arc<SqliteStore>>,
>;

/// Executes an HTTP plan lifecycle request through the native coordinator.
#[test]
#[allow(clippy::too_many_lines)] // Covers typed HTTP dispatch through native worker execution.
fn http_plan_start_runs_the_native_fake_provider_worker() {
    let root = agentic_fixture_root("plan-http");
    let project_store = Arc::new(SqliteStore::open(root.join("project.sqlite")).expect("project"));
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&project_store)),
        StoreOperationProvider::new(Arc::clone(&project_store)),
    );
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(root.join("session.sqlite"))
            .expect("session store"),
    );
    let upstream_models = upstream_model_catalog("local", &["wire-http-plan-model"]);
    let service = ApplicationService::new(backend)
        .with_session_store(session_store.clone())
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
        principal: Some(scriberig_app::Principal::new("http-plan-test")),
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
            goal: "inspect parser behavior through HTTP".to_owned(),
            nodes: vec![PlanNodeDraft {
                key: "inspect".to_owned(),
                parent_index: None,
                title: "Inspect parser".to_owned(),
                purpose: "Summarize parser behavior.".to_owned(),
                expected_result: None,
                semantic_scope: Vec::new(),
                assignment: Some(ModelAssignment {
                    provider_id: "local".to_owned(),
                    model_id: "wire-http-plan-model".to_owned(),
                    thinking_level: Some("high".to_owned()),
                }),
            }],
            dependencies: Vec::new(),
        },
    )
    .expect("create plan");
    let provider = rpi_ai::providers::faux::FauxScript::new().with_text("HTTP worker completed.");
    let runner = NativeTuiAgentRunner {
        providers: std::collections::BTreeMap::from([(
            crate::native_tui_agent::provider_config::catalog::connection_identity(
                "local",
                "wire-http-plan-model",
            ),
            NativeScenario::Faux(provider.clone()),
        )]),
        model: "http-plan-model".to_owned(),
        upstream_credentials: None,
        upstream_enabled: false,
        model_catalog: model_catalog_from_models(upstream_models),
        model_config: std::sync::Mutex::new((rpi_model_runtime::ModelsConfig::default(), None)),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
        continuations: std::sync::Mutex::default(),
    };
    let handler = move |service: &&ApplicationService<HttpBackend>,
                        request_context: &ApplicationContext,
                        request: &scriberig_app::PlanLifecycleRequest,
                        action: scriberig_app::PlanLifecycleAction| {
        scriberig_tui::AgentCommandRunner::<HttpBackend>::plan_lifecycle(
            &runner,
            *service,
            request_context,
            request,
            action,
        )
    };
    let response = scriberig_server::Dispatcher::for_principal(
        &service,
        scriberig_app::Principal::new("http-plan-test"),
    )
    .dispatch_with_plan_lifecycle(
        scriberig_protocol::RequestEnvelope {
            version: scriberig_protocol::ProtocolVersion::default(),
            correlation_id: "http-plan-start".to_owned(),
            request: scriberig_protocol::Request::PlanLifecycle(
                scriberig_protocol::SessionPlanLifecycleRequest {
                    session: scriberig_protocol::SessionReference {
                        id: session_id.to_string(),
                    },
                    plan: plan.id.to_string(),
                    expected_revision: plan.revision,
                    action: scriberig_protocol::SessionPlanAction::Start,
                },
            ),
        },
        Some(&handler),
    );
    let scriberig_protocol::Response::Plan(updated) = response.response else {
        panic!("HTTP plan start should return an updated plan: {response:?}");
    };
    assert_eq!(
        updated.status,
        scriberig_protocol::SessionPlanStatus::Completed
    );
    let durable = session_store
        .load(session_id)
        .expect("load durable session");
    let request = durable
        .as_ref()
        .and_then(|session| {
            session.branches.iter().find_map(|branch| {
                branch
                    .model_invocations
                    .iter()
                    .find_map(|invocation| invocation.request.as_ref())
            })
        })
        .expect("captured upstream request");
    assert_eq!(request.model, "wire-http-plan-model");
    assert_eq!(request.thinking_level.as_deref(), Some("high"));
    assert!(
        durable
            .is_some_and(|value| { value.branches.iter().any(|branch| branch.result.is_some()) })
    );
    drop(handler);
    drop(service);
    drop(session_store);
    drop(project_store);
    std::fs::remove_dir_all(root).expect("remove fixture");
}
