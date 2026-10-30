//! Verifies HTTP mapping for application-owned migration detail.

use super::{
    HttpState, allowed_origin, http_migration::migration, http_protocol::dispatch,
    project_routes::direct,
};
use crate::development::DevelopmentFixture;
use crate::server::ApplicationService;
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, InspectRequest, InspectResult, MigrationDetail, RuntimeCapability,
};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use serde_json::Value;

#[path = "http_artifact_tests.rs"]
mod artifact_tests;
#[path = "http_document_tests.rs"]
mod document_tests;
#[path = "http_dwg_tests.rs"]
mod dwg_tests;
#[path = "http_qgis_tests.rs"]
mod qgis_tests;
#[path = "http_spreadsheet_tests.rs"]
mod spreadsheet_tests;

/// Supplies one migration detail to the HTTP adapter test.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal operations because this fixture only tests HTTP mapping.
    ///
    /// - `_context`: Application authorization context.
    /// - `_request`: Universal request, unused by this fixture.
    ///
    /// Never returns a result.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "HTTP fixture backend".to_owned(),
        })
    }
}

impl ApplicationService for Fixture {
    /// Rejects inspection because this test only exercises migration detail.
    ///
    /// - `_context`: Request authorization context.
    /// - `_request`: Inspection request, intentionally unsupported here.
    ///
    /// Never returns an inspection result in this fixture.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        _request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "inspect".to_owned(),
        })
    }

    /// Returns no capabilities for the focused adapter test.
    ///
    /// An empty capability list.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        vec![
            RuntimeCapability::new(
                scriberig_app::CapabilityId::new("agent_plan").expect("plan capability"),
                "durable plan lifecycle",
                true,
            ),
            RuntimeCapability::new(
                scriberig_app::CapabilityId::new("agent_start").expect("agent capability"),
                "agent execution",
                true,
            ),
        ]
    }

    /// Rejects operation status because it is outside this adapter test.
    ///
    /// - `_operation_id`: Operation identifier, intentionally unsupported here.
    ///
    /// Never returns an operation status in this fixture.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn operation_status(
        &self,
        _operation_id: &str,
    ) -> Result<scriberig_protocol::Operation, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "operation status".to_owned(),
        })
    }

    /// Returns one canonical migration detail.
    ///
    /// - `_context`: Request authorization context.
    /// - `migration_id`: Migration identifier to echo in the detail.
    ///
    /// One deterministic migration detail.
    ///
    /// This fixture does not return an error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn migration_detail(
        &self,
        _context: &ApplicationContext,
        migration_id: &str,
    ) -> Result<MigrationDetail, ApplicationError> {
        Ok(MigrationDetail {
            id: migration_id.to_owned(),
            base: SnapshotId::new([1; 32]),
            result: SnapshotId::new([2; 32]),
            deltas: vec![scriberig_app::MigrationDelta {
                entity: EntityId::from_bytes([3; 16]),
                before: Some(RevisionId::new([4; 32])),
                after: Some(RevisionId::new([5; 32])),
            }],
        })
    }
}

/// Verifies the browser migration shape preserves canonical snapshot details.
#[test]
fn migration_route_maps_canonical_detail() {
    let state = HttpState {
        service: Fixture,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        agent_execution_handler: None,
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let response = migration(&state, r#"{"migrationId":"migration-1"}"#).expect("response");
    let response: Value = response;
    assert_eq!(response["migrationId"], "migration-1");
    assert_eq!(response["changes"][0]["kind"], "modified");
    assert_eq!(
        response["changes"][0]["entityId"],
        "ent_03030303030303030303030303030303"
    );
}

/// Verifies operation-segment requests become typed dispatcher envelopes.
#[test]
fn typed_operation_route_preserves_correlation() {
    let state = HttpState {
        service: Fixture,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        agent_execution_handler: None,
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let response =
        dispatch(&state, "capabilities", "{}", "request-42".to_owned()).expect("typed response");
    assert_eq!(response["correlation_id"], "request-42");
    assert!(response["response"]["Capabilities"].is_object());
}

/// Verifies the HTTP plan lifecycle route invokes its trusted runtime hook.
#[test]
fn typed_plan_lifecycle_route_invokes_runtime_handler() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = std::sync::Arc::clone(&calls);
    let state = HttpState {
        service: Fixture,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        agent_execution_handler: None,
        plan_lifecycle_handler: Some(std::sync::Arc::new(
            move |_service, _context, request, action| {
                assert_eq!(request.expected_revision, 7);
                assert_eq!(action, scriberig_app::PlanLifecycleAction::Start);
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Err(scriberig_app::ApplicationError::UnsupportedOperation {
                    operation: "test worker runtime".to_owned(),
                })
            },
        )),
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let request = scriberig_protocol::SessionPlanLifecycleRequest {
        session: scriberig_protocol::SessionReference {
            id: EntityId::from_bytes([2; 16]).to_string(),
        },
        plan: EntityId::from_bytes([3; 16]).to_string(),
        expected_revision: 7,
        action: scriberig_protocol::SessionPlanAction::Start,
    };
    let body = serde_json::to_string(&request).expect("encode lifecycle request");
    let response = dispatch(&state, "plan_lifecycle", &body, "plan-start".to_owned())
        .expect("typed protocol response");
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(response["correlation_id"], "plan-start");
    assert_eq!(
        response["response"]["Error"]["code"],
        "UnsupportedOperation"
    );
}

/// Verifies HTTP agent requests reach the injected shared execution runtime.
#[test]
fn typed_agent_route_invokes_runtime_handler() {
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = std::sync::Arc::clone(&calls);
    let state = HttpState {
        service: Fixture,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        agent_execution_handler: Some(std::sync::Arc::new(
            move |_, _context, request, accepted| {
                assert!(accepted.is_none());
                assert_eq!(request.session, EntityId::from_bytes([8; 16]));
                assert_eq!(request.prompt, "inspect the selected project");
                observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(scriberig_protocol::Operation {
                    reference: scriberig_protocol::OperationReference {
                        id: "agent-runtime-1".to_owned(),
                    },
                    kind: scriberig_protocol::operation::OperationKind::Agent,
                    status: scriberig_protocol::OperationStatus::Completed,
                    progress: None,
                    result_snapshot: None,
                    result_handles: Vec::new(),
                    agent_result: None,
                    error: None,
                })
            },
        )),
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let body = serde_json::json!({
        "session": {"id": EntityId::from_bytes([8; 16]).to_string()},
        "prompt": "inspect the selected project"
    })
    .to_string();
    let response = dispatch(&state, "agent_start", &body, "agent-start".to_owned())
        .expect("typed agent response");
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(response["correlation_id"], "agent-start");
    assert_eq!(
        response["response"]["AgentAccepted"]["operation"]["operation"]["status"],
        "Completed"
    );
}

/// Verifies HTTP preference operations carry the trusted local principal.
#[test]
fn typed_preference_operations_use_the_http_principal() {
    let state = HttpState {
        service: scriberig_app::ApplicationService::new(Fixture),
        principal: scriberig_app::Principal::new("http-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        agent_execution_handler: None,
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let response = dispatch(
        &state,
        "agent_presentation_preferences",
        "{}",
        "preferences-read".to_owned(),
    )
    .expect("read shared preferences");
    assert_eq!(
        response["response"]["AgentPresentationPreferences"]["revision"],
        0
    );

    let response = dispatch(
        &state,
        "set_agent_presentation_preferences",
        r#"{"expected_revision":0,"preferences":{"reasoning":"shown","tool_invocation":"detailed","tool_output":"full","prompt":"shown"}}"#,
        "preferences-write".to_owned(),
    )
    .expect("update shared preferences");
    assert_eq!(
        response["response"]["AgentPresentationPreferences"]["revision"],
        1
    );
}

/// Verifies unknown operation identifiers fail before application dispatch.
#[test]
fn typed_operation_route_rejects_unknown_operation() {
    let state = HttpState {
        service: Fixture,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        agent_execution_handler: None,
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let (status, body) =
        dispatch(&state, "unknown", "{}", "request-43".to_owned()).expect_err("unknown operation");
    assert_eq!(status, 404);
    assert_eq!(body["code"], "unsupported_operation");
}

/// Verifies the browser transport's direct project route is dispatched.
#[test]
fn direct_project_route_preserves_project_errors() {
    let state = HttpState {
        service: Fixture,
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        agent_execution_handler: None,
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture: DevelopmentFixture {
            root_entity: EntityId::from_bytes([1; 16]),
            child_entities: Vec::new(),
            snapshot: SnapshotId::new([1; 32]),
        },
    };
    let (_, response) = direct(&state, r#"{"operation":"list","payload":{}}"#)
        .expect_err("project catalog should be unavailable");
    assert_eq!(response["code"], "project_catalog_unavailable");
}

/// Verifies CORS accepts only the configured exact origin.
#[test]
fn cors_origin_policy_rejects_wildcards_and_unknown_origins() {
    let head = "POST /protocol/capabilities HTTP/1.1\r\nOrigin: https://app.example\r\n\r\n";
    assert_eq!(
        allowed_origin(head, Some("https://app.example")).as_deref(),
        Some("https://app.example")
    );
    assert!(allowed_origin(head, Some("https://other.example")).is_none());
    assert!(allowed_origin(head, Some("*")).is_none());
}
