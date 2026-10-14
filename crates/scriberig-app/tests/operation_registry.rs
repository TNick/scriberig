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
    let cancellation = registry.cancellation_flag(&id).expect("cancellation flag");
    assert!(!cancellation.load(std::sync::atomic::Ordering::Acquire));
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
    assert!(cancellation.load(std::sync::atomic::Ordering::Acquire));

    assert_eq!(
        registry.get(&id).expect("record").status,
        OperationStatus::Cancelled
    );
    assert!(matches!(
        events.try_iter().next(),
        Some(ApplicationEvent::OperationStarted { .. })
    ));
}

/// Verifies operation snapshots include current active records for late observers.
#[test]
fn registry_lists_current_records_for_late_observers() {
    let registry = OperationRegistry::new();
    let id = OperationId::new("operation-active").expect("operation id");
    registry
        .start(id.clone(), OperationKind::Agent)
        .expect("start");
    registry
        .transition(&id, OperationStatus::Running)
        .expect("running");

    let records = registry.list();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].reference, id);
    assert_eq!(records[0].status, OperationStatus::Running);
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

/// Verifies agent output is published with its operation correlation handle.
#[test]
fn registry_publishes_agent_output() {
    let registry = OperationRegistry::new();
    let events = registry.subscribe();
    let id = OperationId::new("operation-output").expect("operation id");
    registry
        .start(id.clone(), OperationKind::Agent)
        .expect("start");

    registry
        .publish_agent_output(&id, "partial answer")
        .expect("output");

    assert!(matches!(
        events
            .try_iter()
            .find(|event| matches!(event, ApplicationEvent::AgentOutputProduced { .. })),
        Some(ApplicationEvent::AgentOutputProduced { operation, output })
            if operation == id && output == "partial answer"
    ));
}
