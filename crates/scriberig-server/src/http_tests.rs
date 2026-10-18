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

/// Supplies one migration detail to the HTTP adapter test.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects universal operations because this fixture only tests HTTP mapping.
    ///
    /// # Parameters
    /// - `_context`: Application authorization context.
    /// - `_request`: Universal request, unused by this fixture.
    ///
    /// # Returns
    /// Never returns a result.
    ///
    /// # Errors
    /// Always returns unsupported-operation.
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
    /// # Parameters
    /// - `_context`: Request authorization context.
    /// - `_request`: Inspection request, intentionally unsupported here.
    ///
    /// # Returns
    /// Never returns an inspection result in this fixture.
    ///
    /// # Errors
    /// Always returns unsupported-operation.
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
    /// # Returns
    /// An empty capability list.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        Vec::new()
    }

    /// Rejects operation status because it is outside this adapter test.
    ///
    /// # Parameters
    /// - `_operation_id`: Operation identifier, intentionally unsupported here.
    ///
    /// # Returns
    /// Never returns an operation status in this fixture.
    ///
    /// # Errors
    /// Always returns unsupported-operation.
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
    /// # Parameters
    /// - `_context`: Request authorization context.
    /// - `migration_id`: Migration identifier to echo in the detail.
    ///
    /// # Returns
    /// One deterministic migration detail.
    ///
    /// # Errors
    /// This fixture does not return an error.
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

/// Verifies HTTP preference operations carry the trusted local principal.
#[test]
fn typed_preference_operations_use_the_http_principal() {
    let state = HttpState {
        service: scriberig_app::ApplicationService::new(Fixture),
        principal: scriberig_app::Principal::new("http-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
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
