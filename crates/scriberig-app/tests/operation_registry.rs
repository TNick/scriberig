//! Verifies application operation lifecycle state and event publication.

use scriberig_app::{
    ApplicationError, ApplicationEvent, OperationId, OperationKind, OperationProgress,
    OperationRegistry, OperationStatus,
};

/// Verifies lifecycle transitions, cancellation, and event delivery.
#[test]
fn registry_tracks_lifecycle_and_publishes_events() {
    let registry = OperationRegistry::new();
    let events = registry.subscribe();
    let id = OperationId::new("operation-1").expect("operation id");

    registry
        .start(id.clone(), OperationKind::Inspect)
        .expect("start");
    registry
        .transition(&id, OperationStatus::Running)
        .expect("running");
    registry
        .progress(
            &id,
            OperationProgress {
                status: OperationStatus::Running,
                completed: Some(1),
                total: Some(2),
                phase: Some("inspect".to_owned()),
            },
        )
        .expect("progress");
    registry.cancel(&id).expect("cancel");

    assert_eq!(
        registry.get(&id).expect("record").status,
        OperationStatus::Cancelled
    );
    assert!(matches!(
        events.try_iter().next(),
        Some(ApplicationEvent::OperationStarted { .. })
    ));
}

/// Verifies failed operations retain the structured diagnostic they publish.
#[test]
fn registry_retains_failure_details() {
    let registry = OperationRegistry::new();
    let id = OperationId::new("operation-failed").expect("operation id");
    registry
        .start(id.clone(), OperationKind::Inspect)
        .expect("start");
    registry
        .fail(
            &id,
            ApplicationError::InvalidRequest {
                operation: "inspect".to_owned(),
                message: "invalid target".to_owned(),
            },
        )
        .expect("fail");

    let record = registry.get(&id).expect("record");
    assert_eq!(record.status, OperationStatus::Failed);
    assert!(matches!(
        record.error,
        Some(ApplicationError::InvalidRequest { .. })
    ));
}
