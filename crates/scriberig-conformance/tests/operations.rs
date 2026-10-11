//! Verifies operation lifecycle stability through the public client boundary.

use scriberig_app::{OperationKind, OperationProgress, OperationRegistry};
use scriberig_client::Client;
use scriberig_conformance::fixture;
use scriberig_core::OperationId;
use scriberig_protocol::{OperationReference, OperationStatus as ProtocolStatus};
use scriberig_server::InProcessTransport;

/// Verifies one operation handle reaches a terminal state through transport.
#[test]
fn operation_identity_and_terminal_state_survive_transport() {
    let client = Client::new(InProcessTransport::new(fixture()));
    let operation = client
        .operation_status(
            OperationReference {
                id: "op-1".to_owned(),
            }
            .id,
        )
        .expect("operation status");
    let operation = operation.operation;
    assert_eq!(operation.reference.id, "op-1");
    assert_eq!(operation.status, ProtocolStatus::Completed);
    assert_eq!(operation.progress.as_ref().expect("progress").percent, 100);
    assert_eq!(
        operation
            .result_snapshot
            .as_ref()
            .expect("result snapshot")
            .id,
        fixture().snapshot.to_string()
    );

    let reconnect = Client::new(InProcessTransport::new(fixture()));
    assert_eq!(
        reconnect
            .operation_status("op-1")
            .expect("status after reconnect")
            .operation,
        operation
    );
}

/// Verifies active and terminal operation states remain distinct.
#[test]
fn operation_lifecycle_states_are_stable_and_terminal() {
    let client = Client::new(InProcessTransport::new(fixture()));
    let cases = [
        ("op-queued", ProtocolStatus::Queued),
        ("op-running", ProtocolStatus::Running),
        ("op-failed", ProtocolStatus::Failed),
        ("op-cancelled", ProtocolStatus::Cancelled),
    ];

    for (id, expected_status) in cases {
        let operation = client
            .operation_status(id)
            .expect("operation status")
            .operation;
        assert_eq!(operation.reference.id, id);
        assert_eq!(operation.status, expected_status);
        assert!(operation.result_snapshot.is_none());
        assert_eq!(
            operation.error.is_some(),
            matches!(
                &expected_status,
                ProtocolStatus::Failed | ProtocolStatus::Cancelled
            )
        );
    }

    let cancelled = client
        .cancel_operation("op-1")
        .expect("cancel operation")
        .operation;
    assert_eq!(cancelled.reference.id, "op-1");
    assert_eq!(cancelled.status, ProtocolStatus::Cancelled);
    assert!(cancelled.error.is_some());
}

/// Verifies application operation records publish stable lifecycle events.
#[test]
fn application_registry_preserves_results_and_event_handles() {
    let registry = OperationRegistry::new();
    let events = registry.subscribe();
    let id = OperationId::new("conformance").expect("operation id");
    registry
        .start(id.clone(), OperationKind::Query)
        .expect("start operation");
    registry
        .transition(&id, scriberig_app::OperationStatus::Running)
        .expect("run operation");
    registry
        .progress(
            &id,
            OperationProgress {
                status: scriberig_app::OperationStatus::Running,
                completed: Some(1),
                total: Some(2),
                phase: Some("query".to_owned()),
            },
        )
        .expect("progress operation");
    let completed = registry
        .complete(&id, vec!["entity:result".to_owned()])
        .expect("complete operation");

    assert_eq!(completed.reference, id);
    assert_eq!(completed.status, scriberig_app::OperationStatus::Completed);
    assert_eq!(completed.result_handles, vec!["entity:result"]);
    assert!(registry.get(&id).is_ok());
    assert!(matches!(
        events.try_iter().last(),
        Some(scriberig_app::ApplicationEvent::OperationCompleted { .. })
    ));
}
