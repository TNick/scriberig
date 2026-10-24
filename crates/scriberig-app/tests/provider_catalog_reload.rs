//! Verifies host-invoked catalog reload keeps selection, provenance, and
//! credential safety coherent without a process restart.

#[path = "session_runtime_support.rs"]
mod support;
use support::{Fixture, session_context};

use rpi_ai::{Api, Model};
use scriberig_app::{
    AgentCommandRequest, AgentConsoleBackend, ApplicationError, ApplicationService,
    PlanAssignmentRequest, PlanCreateRequest, PlanNodeDraft, SessionBackend,
    SessionModelSelectionRequest, SessionRequest, SessionThinkingSelectionRequest,
};
use scriberig_core::EntityId;

/// Credential material that must never reach an authorized read or session state.
const SENTINEL: &str = "reload-credential-sentinel";

/// Builds one upstream catalog row for reload fixtures.
///
/// - `provider_id`: Upstream provider identity.
/// - `model_id`: Provider-native model identity.
/// - `endpoint`: Resolved provider endpoint for this row.
///
/// The upstream model row published into the shared catalog.
/// # Returns
/// The credential-free upstream model row for the fixture.
fn model_row(provider_id: &str, model_id: &str, endpoint: &str) -> Model {
    Model::new(
        model_id,
        model_id,
        Api::OpenaiCompletions,
        provider_id,
        endpoint,
    )
}

/// Builds one credential-free upstream plan assignment.
///
/// - `provider_id`: Upstream provider identity.
/// - `model_id`: Provider-native model identity.
///
/// The plan-node assignment for the reload fixtures.
/// # Returns
/// The upstream provider/model assignment with no thinking override.
fn model_assignment(provider_id: &str, model_id: &str) -> scriberig_session::ModelAssignment {
    scriberig_session::ModelAssignment {
        provider_id: provider_id.to_owned(),
        model_id: model_id.to_owned(),
        thinking_level: None,
    }
}

/// Reports whether any credential sentinel appears in a serialized value.
///
/// - `value`: Value to serialize and inspect.
///
/// `true` when the serialized form contains the fixture sentinel.
/// # Returns
/// Whether the serialized form leaked the sentinel.
fn serialized_contains_credential<T: serde::Serialize>(value: &T) -> bool {
    serde_json::to_string(value)
        .expect("serializable fixture")
        .contains(SENTINEL)
}

/// Verifies one reload publishes a new generation that every component sharing
/// the catalog handle observes without restarting the process.
#[test]
fn reload_publishes_a_new_generation_observed_by_shared_components() {
    let service = ApplicationService::new(Fixture);
    let shared_catalog = service.model_catalog();
    let before = service.model_catalog_generation();
    assert_eq!(before, scriberig_app::catalog_generation(&shared_catalog));

    let after = service.reload_model_catalog(vec![model_row(
        "provider-a",
        "model-a",
        "https://provider-a.example.test/v1",
    )]);

    assert!(after.revision() > before.revision());
    assert_ne!(before, scriberig_app::catalog_generation(&shared_catalog));
    assert_eq!(after, scriberig_app::catalog_generation(&shared_catalog));
    assert_eq!(service.model_catalog_generation(), after);
    let library = service
        .agent_model_library(&session_context())
        .expect("authorized model read after reload");
    assert_eq!(library.len(), 1);
    assert_eq!(library[0].provider_id, "provider-a");
    assert_eq!(library[0].model_id, "model-a");
    assert_eq!(service.upstream_models().len(), 1);

    let unchanged = service.reload_model_catalog(vec![model_row(
        "provider-a",
        "model-a",
        "https://provider-a.example.test/v1",
    )]);
    assert_eq!(unchanged, after);
    assert_eq!(service.model_catalog_generation(), after);
}

/// Verifies a reload that removes the selected pair fails closed for that
/// selection instead of silently falling back, and that the replacement pair
/// is selectable in the same process.
#[test]
fn reload_fails_closed_for_a_stale_selection_and_accepts_the_new_generation() {
    let service = ApplicationService::new(Fixture).with_upstream_models(vec![
        model_row("local", "native-a", "https://local.example.test/v1"),
        model_row("local", "native-b", "https://local.example.test/v1"),
    ]);
    let context = session_context();
    let session = EntityId::from_bytes([41; 16]);
    service
        .start_agent(&context, session, "seed session")
        .expect("create session");
    service
        .set_session_model(
            &context,
            &SessionModelSelectionRequest {
                session,
                expected_revision: 0,
                provider_id: "local".to_owned(),
                model_id: "native-a".to_owned(),
            },
        )
        .expect("select the first model");

    let reloaded = service.reload_model_catalog(vec![model_row(
        "local",
        "native-b",
        "https://local.example.test/v1",
    )]);

    assert!(reloaded.revision() > 0);
    let stale = service.set_session_thinking_level(
        &context,
        &SessionThinkingSelectionRequest {
            session,
            expected_revision: 1,
            thinking_level: "high".to_owned(),
        },
    );
    assert!(
        matches!(
            &stale,
            Err(ApplicationError::InvalidRequest { operation, .. })
                if operation.as_str() == "session model selection"
        ),
        "a selection naming a removed model must not resolve or fall back: {stale:?}"
    );
    let unchanged = SessionBackend::read_session(&service, &context, &SessionRequest { session })
        .expect("read session after the rejected write");
    assert_eq!(unchanged.execution.model_id.as_deref(), Some("native-a"));

    let current = service
        .set_session_model(
            &context,
            &SessionModelSelectionRequest {
                session,
                expected_revision: 1,
                provider_id: "local".to_owned(),
                model_id: "native-b".to_owned(),
            },
        )
        .expect("select the reloaded model");
    assert_eq!(current.model_id.as_deref(), Some("native-b"));
    let library = service
        .agent_model_library(&context)
        .expect("authorized model read after reload");
    assert_eq!(
        library
            .iter()
            .map(|row| row.model_id.as_str())
            .collect::<Vec<_>>(),
        vec!["native-b"]
    );
}

/// Verifies a host reload leaves a plan node's stale assignment unresolvable at
/// the durable plan boundary and lets an operator move it to a current pair.
#[test]
fn reload_rejects_a_stale_plan_assignment_and_accepts_the_new_generation() {
    let service = ApplicationService::new(Fixture).with_upstream_models(vec![
        model_row("local", "native-a", "https://local.example.test/v1"),
        model_row("local", "native-b", "https://local.example.test/v1"),
    ]);
    let context = session_context();
    let session = EntityId::from_bytes([43; 16]);
    service
        .start_agent(&context, session, "plan anchor")
        .expect("seed session");
    let branch = SessionBackend::read_session(&service, &context, &SessionRequest { session })
        .expect("read the seeded session")
        .branches[0]
        .id;
    let plan = service
        .create_plan(
            &context,
            &PlanCreateRequest {
                session,
                anchor: scriberig_session::ContinuationAnchor::BranchHead { branch },
                goal: "finish the feature".to_owned(),
                nodes: vec![PlanNodeDraft {
                    key: "design".to_owned(),
                    parent_index: None,
                    title: "Design".to_owned(),
                    purpose: "define the interface".to_owned(),
                    expected_result: None,
                    semantic_scope: Vec::new(),
                    assignment: Some(model_assignment("local", "native-a")),
                }],
                dependencies: Vec::new(),
            },
        )
        .expect("plan assigned to a current model");

    let reloaded = service.reload_model_catalog(vec![model_row(
        "local",
        "native-b",
        "https://local.example.test/v1",
    )]);
    assert!(reloaded.revision() > 0);

    let rejected = service.set_plan_node_assignment(
        &context,
        &PlanAssignmentRequest {
            session,
            plan: plan.id,
            node: plan.nodes[0].id,
            expected_revision: plan.revision,
            assignment: Some(model_assignment("local", "native-a")),
        },
    );
    assert!(
        matches!(
            &rejected,
            Err(ApplicationError::InvalidRequest { operation, .. })
                if operation.as_str() == "plan"
        ),
        "a removed pair may not be stamped onto a node again: {rejected:?}"
    );
    let retained = SessionBackend::read_session(&service, &context, &SessionRequest { session })
        .expect("read the session after the rejected plan write");
    let retained_plan = retained
        .plans
        .iter()
        .find(|candidate| candidate.id == plan.id)
        .expect("the plan survives the rejected write");
    assert_eq!(retained_plan.revision, plan.revision);
    assert_eq!(
        retained_plan.nodes[0]
            .assignment
            .as_ref()
            .expect("stored assignment")
            .model_id,
        "native-a",
        "a rejected plan write must leave the stored assignment inspectable"
    );

    let reassigned = service
        .set_plan_node_assignment(
            &context,
            &PlanAssignmentRequest {
                session,
                plan: plan.id,
                node: plan.nodes[0].id,
                expected_revision: plan.revision,
                assignment: Some(model_assignment("local", "native-b")),
            },
        )
        .expect("reassign the node to the new generation");
    assert_eq!(
        reassigned.nodes[0]
            .assignment
            .as_ref()
            .expect("reassigned node")
            .model_id,
        "native-b"
    );
}

/// Verifies a reload keeps captured request provenance inspectable and keeps
/// credential material out of the catalog view, the session graph, and the
/// authorized historical detail.
#[test]
fn reload_keeps_detail_inspectable_and_persists_no_credential_material() {
    let credential_endpoint = format!("https://{SENTINEL}.example.test/v1");
    let service = ApplicationService::new(Fixture);
    let first_generation = service.reload_model_catalog(vec![{
        let mut row = model_row("provider-a", "model-a", credential_endpoint.as_str());
        row.headers = Some(std::collections::BTreeMap::from([(
            "authorization".to_owned(),
            SENTINEL.to_owned(),
        )]));
        row
    }]);
    let handle = service
        .start_local_agent(&AgentCommandRequest {
            session: EntityId::from_bytes([42; 16]),
            branch: None,
            anchor: None,
            prompt: "review the bounded request".to_owned(),
            selection: None,
        })
        .expect("start local agent");
    let captured = vec![scriberig_session::ModelRequestProjection {
        ordinal: 0,
        model: "model-a".to_owned(),
        provider_id: Some("provider-a".to_owned()),
        provider_profile: None,
        thinking_level: Some("high".to_owned()),
        profile: None,
        active_components: Vec::new(),
        context_fingerprint: None,
        request_fingerprint: None,
        model_execution_configuration: None,
        messages: vec![scriberig_session::ModelRequestMessage {
            role: "user".to_owned(),
            content: "safe request".to_owned(),
        }],
        full_messages: None,
        tools: Vec::new(),
        full_tools: None,
        content_complete: false,
        applied_settings: None,
        applied_settings_fingerprint: None,
        truncated: false,
    }];
    service
        .complete_local_agent_with_evidence(
            &handle,
            "answer",
            Vec::new(),
            Vec::new(),
            captured.clone(),
        )
        .expect("persist the captured request");

    let context = session_context();
    let library = service
        .agent_model_library(&context)
        .expect("authorized model read");
    assert!(
        !serialized_contains_credential(&library),
        "the authorized model read exposed catalog credential material"
    );

    // A second reload replaces the generation the request was captured against.
    let second_generation = service.reload_model_catalog(Vec::new());
    assert!(second_generation.revision() > first_generation.revision());

    let durable = SessionBackend::read_session(
        &service,
        &context,
        &SessionRequest {
            session: handle.session,
        },
    )
    .expect("read the session graph after reload");
    assert!(!serialized_contains_credential(&durable));
    let invocation_id = durable.branches[0].model_invocations[0].id;
    let detail = AgentConsoleBackend::model_request_detail(
        &service,
        &context,
        handle.session,
        handle.root_branch,
        invocation_id,
    )
    .expect("historical detail after reload");
    assert_eq!(detail, captured[0]);
    assert_eq!(detail.provider_id.as_deref(), Some("provider-a"));
    assert_eq!(detail.model, "model-a");
    assert!(!serialized_contains_credential(&detail));
    assert_eq!(
        service
            .agent_model_library(&context)
            .expect("model read after provider removal"),
        Vec::new()
    );
}
