//! Covers authoritative TUI session refresh and request failure behavior.

use super::*;
use crate::inspector::LoadState;

/// Verifies a completed agent operation reloads its durable session branch.
#[test]
fn completed_agent_operation_refreshes_session() {
    let operation = scriberig_core::OperationId::new("session-refresh").expect("operation");
    let session = scriberig_core::EntityId::from_bytes([21; 16]);
    let branch = scriberig_core::EntityId::from_bytes([22; 16]);
    let mut state = TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: LoadState::Ready(crate::view::session::SessionModel::default()),
        ..TuiState::default()
    };
    state.agent_handles.push(crate::service::AgentHandle {
        operation: operation.clone(),
        session,
        branch: Some(branch),
        workspace: None,
    });
    let mut record =
        scriberig_app::OperationRecord::queued(operation, scriberig_app::OperationKind::Agent);
    record.status = scriberig_app::OperationStatus::Completed;

    let effects = reduce(
        &mut state,
        Action::OperationObserved(crate::view::operations::OperationModel::from_record(
            &record,
        )),
    );

    assert!(matches!(
        effects.as_slice(),
        [Effect::ReadSession {
            request_id: 1,
            session: requested_session,
            branch: Some(requested_branch),
        }] if *requested_session == session && *requested_branch == branch
    ));
    assert!(matches!(
        state.session,
        crate::inspector::LoadState::Loading { request_id: 1 }
    ));
}

/// Verifies completion does not race an already pending initial session read.
#[test]
fn completed_operation_preserves_pending_initial_session_read() {
    let operation = scriberig_core::OperationId::new("session-loading").expect("operation");
    let session = scriberig_core::EntityId::from_bytes([23; 16]);
    let mut state = TuiState {
        primary_view: crate::selection::PrimaryView::Session,
        session: LoadState::Loading { request_id: 7 },
        ..TuiState::default()
    };
    state.agent_handles.push(crate::service::AgentHandle {
        operation: operation.clone(),
        session,
        branch: None,
        workspace: None,
    });
    let mut record =
        scriberig_app::OperationRecord::queued(operation, scriberig_app::OperationKind::Agent);
    record.status = scriberig_app::OperationStatus::Completed;

    let effects = reduce(
        &mut state,
        Action::OperationObserved(crate::view::operations::OperationModel::from_record(
            &record,
        )),
    );

    assert_eq!(effects, vec![Effect::None]);
    assert!(matches!(
        state.session,
        LoadState::Loading { request_id: 7 }
    ));
}

/// Verifies a failed session read leaves loading state and exposes its diagnostic.
#[test]
fn failed_session_read_replaces_loading_state() {
    let mut state = TuiState {
        session: crate::inspector::LoadState::Loading { request_id: 3 },
        ..TuiState::default()
    };

    reduce(
        &mut state,
        Action::OperationFailed {
            request_id: 3,
            category: "session_read_failed".to_owned(),
            message: "session store unavailable".to_owned(),
        },
    );

    assert!(matches!(
        state.session,
        crate::inspector::LoadState::Failed {
            category,
            message,
            stale: false,
        } if category == "session_read_failed" && message == "session store unavailable"
    ));
}
