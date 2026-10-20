//! Verifies operation cancellation and event observation through the public boundary.

use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationRequest, ApplicationResult, ApplicationService, ApplyRequest, OperationId,
    OperationKind, OperationSummary, RenderRequest, SessionBackend, SnapshotSelector,
};
use scriberig_client::{Client, ClientError};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_query::{
    CollectionRef, DetailLevel, EntityInSnapshot, ListRequest, PageRequest, QueryExpr,
    QueryOrdering, QueryScope, QuerySpec,
};
use scriberig_server::InProcessTransport;

/// Provides an inert backend while the lifecycle registry is exercised.
struct Backend;

impl ApplicationBackend for Backend {
    /// Rejects semantic execution because this test focuses on lifecycle state.
    /// - `_context`: Argument named `_context`.
    /// - `request`: Argument named `request`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute(
        &self,
        _context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        if let ApplicationRequest::Query(specification) = request {
            return Ok(ApplicationResult::Query(scriberig_query::QueryResult {
                scope: specification.scope.clone(),
                page: scriberig_query::Page {
                    items: Vec::new(),
                    next: None,
                },
                information_cost: None,
            }));
        }
        if let ApplicationRequest::List(specification) = request {
            return Ok(ApplicationResult::List(scriberig_query::ListResult {
                subject: specification.subject,
                snapshot: specification.subject.snapshot,
                collection: specification.collection.clone(),
                page: scriberig_query::Page {
                    items: Vec::new(),
                    next: None,
                },
            }));
        }
        if let ApplicationRequest::Apply(_) = request {
            return Ok(ApplicationResult::Apply(OperationSummary {
                handle: "candidate-1".to_owned(),
                snapshot: None,
                revision: None,
                branch: None,
                diff: None,
                history: None,
                rendered: None,
                validation: None,
                validation_profile: None,
                validation_diagnostics: Vec::new(),
                review_details: Vec::new(),
                conflicts: Vec::new(),
            }));
        }
        if let ApplicationRequest::Render(_) = request {
            return Ok(ApplicationResult::Render(OperationSummary {
                handle: "rendered-1".to_owned(),
                snapshot: None,
                revision: None,
                branch: None,
                diff: None,
                history: None,
                rendered: None,
                validation: None,
                validation_profile: None,
                validation_diagnostics: Vec::new(),
                review_details: Vec::new(),
                conflicts: Vec::new(),
            }));
        }
        Err(ApplicationError::UnsupportedOperation {
            operation: "test backend".to_owned(),
        })
    }

    /// Returns a deterministic operation identity for agent command tests.
    ///
    /// - `_context`: Application context, unused by this fixture.
    /// - `_request`: Agent command, unused by this fixture.
    ///
    /// A deterministic operation identifier.
    ///
    /// Returns an internal error only if the fixed identifier is invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        _request: &AgentCommandRequest,
    ) -> Result<OperationId, ApplicationError> {
        OperationId::new("agent-command-1").map_err(|error| ApplicationError::Internal {
            message: error.to_string(),
        })
    }
}

impl SessionBackend for Backend {
    /// Rejects session reads because this test only exercises command dispatch.
    ///
    /// - `_context`: Application context, unused by this fixture.
    /// - `_request`: Session request, intentionally unsupported here.
    ///
    /// Never returns a session in this fixture.
    ///
    /// Always returns unsupported-operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        _context: &ApplicationContext,
        _request: &scriberig_app::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "session read".to_owned(),
        })
    }

    /// Returns a deterministic operation identity for agent command tests.
    ///
    /// - `_context`: Application context, unused by this fixture.
    /// - `_request`: Agent command, unused by this fixture.
    ///
    /// A deterministic operation identifier.
    ///
    /// Returns an internal error only if the fixed identifier is invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit_agent_command(
        &self,
        _context: &ApplicationContext,
        _request: &AgentCommandRequest,
    ) -> Result<OperationId, ApplicationError> {
        OperationId::new("agent-command-1").map_err(|error| ApplicationError::Internal {
            message: error.to_string(),
        })
    }
}

/// Verifies status, cancellation, and event polling share one registry.
#[test]
fn lifecycle_is_visible_through_client_and_server() {
    let service = ApplicationService::new(Backend);
    let operation = OperationId::new("lifecycle-1").expect("operation");
    service
        .operations()
        .start(operation.clone(), OperationKind::Inspect)
        .expect("start");

    let client = Client::new(InProcessTransport::new(service));
    let status = client
        .operation_status(operation.to_string())
        .expect("status");
    assert_eq!(
        status.operation.status,
        scriberig_protocol::OperationStatus::Queued
    );

    let cancelled = client
        .cancel_operation(operation.to_string())
        .expect("cancel");
    assert_eq!(
        cancelled.operation.status,
        scriberig_protocol::OperationStatus::Cancelled
    );
    assert_ne!(client.subscribe(Vec::new()).expect("events").len(), 0);
}

/// Verifies a reconnect cursor excludes events already observed by a client.
#[test]
fn event_cursor_resumes_after_last_observed_event() {
    let service = ApplicationService::new(Backend);
    let operation = OperationId::new("cursor-1").expect("operation");
    service
        .operations()
        .start(operation, OperationKind::Inspect)
        .expect("start");
    let client = Client::new(InProcessTransport::new(service));

    let first = client.subscribe_after(Vec::new(), None).expect("events");
    let cursor = first.last().expect("event").sequence;
    assert_eq!(
        client
            .subscribe_after(Vec::new(), Some(cursor))
            .expect("resumed events")
            .len(),
        0
    );
}

/// Verifies expired cursors request authoritative state refresh.
#[test]
fn expired_event_cursor_is_structured() {
    let service = ApplicationService::new(Backend);
    for index in 0..1_026 {
        let operation = OperationId::new(format!("cursor-expired-{index}")).expect("operation");
        service
            .operations()
            .start(operation, OperationKind::Inspect)
            .expect("start");
    }
    let client = Client::new(InProcessTransport::new(service));
    let error = client
        .subscribe_after(Vec::new(), Some(1))
        .expect_err("expired cursor");
    assert!(matches!(
        error,
        ClientError::Protocol(error)
            if error.code == scriberig_protocol::ProtocolErrorCode::CursorTooOld
    ));
}

/// Verifies remote agent submission joins the shared operation lifecycle.
#[test]
fn agent_command_uses_shared_operation_boundary() {
    let session = scriberig_core::EntityId::from_bytes([1; 16]);
    let client = Client::new(InProcessTransport::new(ApplicationService::new(Backend)));
    let response = client
        .submit_agent_command(session.to_string(), None::<String>, "inspect")
        .expect("agent operation");
    assert_eq!(
        response.operation.kind,
        scriberig_protocol::operation::OperationKind::Agent
    );
    assert_eq!(
        response.operation.status,
        scriberig_protocol::OperationStatus::Running
    );
}

/// Verifies query crosses the boundary while unsupported commands stay typed.
#[test]
fn query_and_command_requests_preserve_structured_errors() {
    let client = Client::new(InProcessTransport::new(ApplicationService::new(Backend)));
    let query = QuerySpec {
        scope: QueryScope::Snapshot(SnapshotId::new([1; 32])),
        expression: QueryExpr::All,
        ordering: QueryOrdering::StableDefault,
        page: PageRequest::default(),
        detail: DetailLevel::Compact,
    };
    let query_result = client.query(query).expect("query result");
    assert_eq!(query_result.page.items.len(), 0);

    let list_result = client
        .list(ListRequest {
            subject: EntityInSnapshot {
                entity: EntityId::from_bytes([2; 16]),
                snapshot: SnapshotId::new([1; 32]),
            },
            collection: CollectionRef {
                owner: EntityId::from_bytes([2; 16]),
                collection_type: "snapshot.entities".to_owned(),
            },
            page: PageRequest::default(),
            detail: DetailLevel::Compact,
        })
        .expect("list result");
    assert_eq!(list_result.page.items.len(), 0);

    let command = client
        .command(
            "apply",
            serde_json::to_value(ApplyRequest {
                base_snapshot: SnapshotId::new([1; 32]),
                branch: None,
                change: "change-1".to_owned(),
            })
            .expect("command payload"),
        )
        .expect("command accepted");
    assert_eq!(
        command.operation.status,
        scriberig_protocol::OperationStatus::Completed
    );
    assert_eq!(command.operation.result_handles, vec!["candidate-1"]);

    let render = client
        .command(
            "render",
            serde_json::to_value(RenderRequest {
                snapshot: SnapshotSelector::Exact(SnapshotId::new([1; 32])),
                view: "markdown".to_owned(),
            })
            .expect("render payload"),
        )
        .expect("render command accepted");
    assert_eq!(render.operation.result_handles, vec!["rendered-1"]);
}
