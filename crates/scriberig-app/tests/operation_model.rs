//! Verifies tracked operation records and structured application failures.

use scriberig_app::{
    ApplicationContext, ApplicationError, ConflictDetails, DiffRequest, OperationId, OperationKind,
    OperationProvider, OperationRecord, OperationStatus, StoreOperationProvider,
};
use scriberig_core::{RevisionId, SnapshotId};
use scriberig_store_memory::MemoryStore;

/// Verifies terminal operation states cannot be changed.
#[test]
fn terminal_operation_is_immutable() {
    let operation = OperationRecord::queued(
        OperationId::new("inspect-1").expect("operation"),
        OperationKind::Inspect,
    )
    .transition(OperationStatus::Running)
    .expect("running")
    .transition(OperationStatus::Completed)
    .expect("completed");
    let error = operation
        .transition(OperationStatus::Failed)
        .expect_err("terminal transition");
    assert!(matches!(error, ApplicationError::OperationFailed { .. }));
}

/// Verifies conflict and ambiguity errors retain semantic handles.
#[test]
fn structured_failures_preserve_handles() {
    let base = RevisionId::new([1; 32]);
    let current = RevisionId::new([2; 32]);
    let snapshot = SnapshotId::new([3; 32]);
    let conflict = ApplicationError::Conflict {
        subject: "entity_1".to_owned(),
        details: Box::new(ConflictDetails {
            base_revision: Some(base),
            current_revision: Some(current),
            ..ConflictDetails::default()
        }),
    };
    assert_eq!(conflict.category(), "conflict");
    let ambiguous = ApplicationError::Ambiguous {
        scope: snapshot.to_string(),
        candidates: vec!["entity_1".to_owned(), "entity_2".to_owned()],
        reason: "duplicate semantic names".to_owned(),
    };
    assert_eq!(ambiguous.category(), "ambiguous");
}

/// Verifies storage-backed diff execution preserves exact snapshot identity.
#[test]
fn storage_diff_is_snapshot_bound() {
    let store = MemoryStore::new();
    let snapshot = store.empty_snapshot();
    let provider = StoreOperationProvider::new(store);
    let result = provider
        .diff(
            &ApplicationContext::empty(),
            &DiffRequest {
                from: snapshot,
                to: snapshot,
            },
        )
        .expect("diff");
    assert_eq!(result.snapshot, Some(snapshot));
    assert!(result.handle.starts_with("diff:"));
}
