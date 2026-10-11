//! Verifies operation cancellation and event observation through the public boundary.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, ApplyRequest, OperationId, OperationKind,
    OperationSummary,
};
use scriberig_client::Client;
use scriberig_core::SnapshotId;
use scriberig_query::{DetailLevel, PageRequest, QueryExpr, QueryOrdering, QueryScope, QuerySpec};
use scriberig_server::InProcessTransport;

/// Provides an inert backend while the lifecycle registry is exercised.
struct Backend;

impl ApplicationBackend for Backend {
    /// Rejects semantic execution because this test focuses on lifecycle state.
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
        if let ApplicationRequest::Apply(_) = request {
            return Ok(ApplicationResult::Apply(OperationSummary {
                handle: "candidate-1".to_owned(),
                snapshot: None,
                revision: None,
                branch: None,
                diff: None,
                history: None,
                rendered: None,
            }));
        }
        Err(ApplicationError::UnsupportedOperation {
            operation: "test backend".to_owned(),
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
}
