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

/// Verifies a transport error marks cached semantic data stale.
#[test]
fn transport_failure_marks_connection_disconnected() {
    let mut state = TuiState::default();
    reduce(
        &mut state,
        Action::InspectFailed {
            request_id: 1,
            category: "transport_unavailable".to_owned(),
            message: "unavailable".to_owned(),
        },
    );
    assert_eq!(
        state.connection,
        crate::state::ConnectionState::Disconnected
    );
    assert!(state.stale);
}

/// Verifies reconnecting schedules an authoritative refresh for the selection.
#[test]
fn reconnect_marks_cache_stale_until_refresh() {
    let mut state = TuiState::default();
    let entity = scriberig_core::EntityId::from_bytes([9; 16]);
    state.connection = crate::state::ConnectionState::Disconnected;
    state.stale = true;
    state.selection = Some(crate::selection::EntitySelection {
        entity,
        revision: None,
        snapshot: None,
    });
    let effects = reduce(&mut state, Action::ConnectionRestored);
    assert_eq!(state.connection, crate::state::ConnectionState::Connected);
    assert!(state.stale);
    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectEntity { entity: selected, .. }] if selected == &entity
    ));
}
