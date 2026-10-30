//! Verifies event-stream disconnects do not cancel observed operations.

use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use scriberig_app::{
    ApplicationContext, ApplicationError, InspectRequest, InspectResult, RuntimeCapability,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_protocol::operation::{OperationKind, OperationStatus};
use scriberig_protocol::{Event, Operation, OperationReference};

use crate::development::DevelopmentFixture;
use crate::http::{HttpState, http_events};
use crate::server::ApplicationService;

/// Supplies stable active-operation state to the SSE disconnect test.
#[derive(Clone)]
struct ActiveOperation {
    /// Whether the independent cancellation endpoint was invoked.
    cancelled: Arc<AtomicBool>,
}

impl ApplicationService for ActiveOperation {
    /// Rejects semantic inspection because this test observes lifecycle events.
    ///
    /// - `_context`: Trusted request context.
    /// - `_request`: Inspection request outside this test's scope.
    ///
    /// Never returns an inspection result.
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
            operation: "inspection".to_owned(),
        })
    }

    /// Returns no optional capabilities for the transport fixture.
    ///
    /// An empty capability list.
    /// # Returns
    /// The operation result.
    fn capabilities(&self) -> Vec<RuntimeCapability> {
        Vec::new()
    }

    /// Reports the observed operation as running until explicit cancellation.
    ///
    /// - `operation_id`: Stable operation identity to inspect.
    ///
    /// Current state for the fixed test operation.
    ///
    /// Returns not-found for any other operation identity.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn operation_status(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        if operation_id != "observe-me" {
            return Err(ApplicationError::NotFound {
                subject: operation_id.to_owned(),
            });
        }
        Ok(Operation {
            reference: OperationReference {
                id: operation_id.to_owned(),
            },
            kind: OperationKind::Agent,
            status: if self.cancelled.load(Ordering::Acquire) {
                OperationStatus::Cancelled
            } else {
                OperationStatus::Running
            },
            progress: None,
            result_snapshot: None,
            result_handles: Vec::new(),
            agent_result: None,
            error: None,
        })
    }

    /// Explicitly cancels the fixed test operation.
    ///
    /// - `operation_id`: Stable operation identity to cancel.
    ///
    /// Cancelled operation state.
    ///
    /// Returns not-found for any other operation identity.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn cancel_operation(&self, operation_id: &str) -> Result<Operation, ApplicationError> {
        if operation_id != "observe-me" {
            return Err(ApplicationError::NotFound {
                subject: operation_id.to_owned(),
            });
        }
        self.cancelled.store(true, Ordering::Release);
        self.operation_status(operation_id)
    }

    /// Returns no events for this lifecycle-only stream fixture.
    ///
    /// - `_event_kinds`: Event filters, unused by this test.
    ///
    /// An empty event history.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn events(&self, _event_kinds: &[String]) -> Result<Vec<Event>, ApplicationError> {
        Ok(Vec::new())
    }
}

/// Proves closing the SSE observer leaves the observed operation running.
#[test]
fn disconnecting_event_observer_does_not_cancel_operation() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let service = ActiveOperation {
        cancelled: Arc::clone(&cancelled),
    };
    assert_eq!(
        service
            .operation_status("observe-me")
            .expect("active operation")
            .status,
        OperationStatus::Running
    );
    let fixture = DevelopmentFixture {
        root_entity: EntityId::from_bytes([1; 16]),
        child_entities: Vec::new(),
        snapshot: SnapshotId::new([2; 32]),
    };
    let state = HttpState {
        service: service.clone(),
        principal: scriberig_app::Principal::new("test-user"),
        allowed_origin: None,
        project_store: None,
        allowed_project_roots: Vec::new(),
        agent_execution_handler: None,
        plan_lifecycle_handler: None,
        provider_readiness_handler: None,
        fixture,
    };
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept event observer");
        http_events::handle_with_lease(
            &mut stream,
            &state,
            "/protocol/events",
            None,
            Duration::from_millis(20),
        )
        .expect("finish bounded observer lease");
    });

    let observer = TcpStream::connect(address).expect("connect event observer");
    let mut observer = BufReader::new(observer);
    observer
        .get_mut()
        .write_all(b"GET /protocol/events HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("request event stream");
    let mut response_line = String::new();
    observer
        .read_line(&mut response_line)
        .expect("read SSE status line");
    assert!(response_line.starts_with("HTTP/1.1 200 OK"));
    loop {
        response_line.clear();
        observer
            .read_line(&mut response_line)
            .expect("read SSE header");
        if response_line == "\r\n" || response_line == "\n" {
            break;
        }
    }
    observer
        .get_ref()
        .shutdown(Shutdown::Both)
        .expect("disconnect observer");
    drop(observer);
    server.join().expect("event handler thread");

    assert!(!cancelled.load(Ordering::Acquire));
    assert_eq!(
        service
            .operation_status("observe-me")
            .expect("operation after disconnect")
            .status,
        OperationStatus::Running,
        "observation disconnect must not change operation lifecycle"
    );
}
