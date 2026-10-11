//! Covers the remaining TUI reducer interaction tests.

use super::*;

/// Verifies cancellation waits for authoritative lifecycle state.
#[test]
fn cancellation_waits_for_authoritative_lifecycle_state() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("cancel").expect("operation id");
    let queued = scriberig_app::OperationRecord::queued(
        operation.clone(),
        scriberig_app::OperationKind::Query,
    );
    reduce(
        &mut state,
        Action::OperationObserved(crate::view::operations::OperationModel::from_record(
            &queued,
        )),
    );
    assert_eq!(
        reduce(
            &mut state,
            Action::OperationCancelRequested(operation.clone())
        ),
        vec![Effect::CancelOperation {
            operation: operation.clone()
        }]
    );
    assert!(state.operations[0].cancellable());
    let cancelled = queued
        .transition(scriberig_app::OperationStatus::Cancelled)
        .expect("cancel transition");
    reduce(
        &mut state,
        Action::OperationObserved(crate::view::operations::OperationModel::from_record(
            &cancelled,
        )),
    );
    assert!(!state.operations[0].cancellable());
}
