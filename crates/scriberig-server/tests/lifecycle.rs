//! Verifies operation cancellation and event observation through the public boundary.

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, OperationId, OperationKind,
};
use scriberig_client::Client;
use scriberig_server::InProcessTransport;

/// Provides an inert backend while the lifecycle registry is exercised.
struct Backend;

impl ApplicationBackend for Backend {
    /// Rejects semantic execution because this test focuses on lifecycle state.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
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
