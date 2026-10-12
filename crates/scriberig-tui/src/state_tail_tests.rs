//! Covers the remaining TUI reducer interaction tests.

use super::super::state_actions::activate_current;
use super::*;

/// Verifies an authoritative progress record reaches the operation view.
#[test]
fn operation_progress_updates_presentation() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("progress").expect("operation id");
    let mut record =
        scriberig_app::OperationRecord::queued(operation, scriberig_app::OperationKind::Query);
    record.status = scriberig_app::OperationStatus::Running;
    record.progress = Some(scriberig_app::OperationProgress {
        status: scriberig_app::OperationStatus::Running,
        completed: Some(3),
        total: Some(8),
        phase: None,
    });
    reduce(
        &mut state,
        Action::OperationObserved(crate::view::operations::OperationModel::from_record(
            &record,
        )),
    );
    assert_eq!(state.operations[0].progress.as_deref(), Some("3/8"));
    assert_eq!(
        state.operations[0].status,
        scriberig_app::OperationStatus::Running
    );
}

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

/// Verifies successful service discovery replaces the unattached placeholder.
#[test]
fn capabilities_loaded_reports_attached_service() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::CapabilitiesLoaded(Vec::new()));
    assert_eq!(state.shell_message, "Semantic service attached.");
    assert_eq!(state.connection, crate::state::ConnectionState::Connected);
}

/// Verifies transient output is discarded after authoritative session refresh.
#[test]
fn authoritative_session_refresh_clears_transient_output() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("durable-result").expect("operation");
    let mut completed =
        scriberig_app::OperationRecord::queued(operation, scriberig_app::OperationKind::Agent);
    completed.status = scriberig_app::OperationStatus::Completed;
    completed.result_handles = vec!["branch-result:durable".to_owned()];
    reduce(
        &mut state,
        Action::AgentOutputObserved("working".to_owned()),
    );
    reduce(
        &mut state,
        Action::OperationObserved(crate::view::operations::OperationModel::from_record(
            &completed,
        )),
    );
    state.session = crate::inspector::LoadState::Loading { request_id: 1 };
    reduce(
        &mut state,
        Action::SessionSucceeded {
            request_id: 1,
            model: crate::view::session::SessionModel::default(),
        },
    );
    assert_eq!(state.agent_output_observations, Vec::<String>::new());
    assert_eq!(
        state.operations[0].result_handles,
        vec!["branch-result:durable".to_owned()]
    );
}

/// Verifies entering the session view requests the selected session entity.
#[test]
fn session_view_requests_selected_entity() {
    let mut state = TuiState::default();
    let session = scriberig_core::EntityId::from_bytes([4; 16]);
    state.selection = Some(crate::selection::EntitySelection {
        entity: session,
        revision: None,
        snapshot: None,
    });

    let effects = reduce(
        &mut state,
        Action::ViewSelected(crate::selection::PrimaryView::Session),
    );

    assert!(matches!(
        effects.as_slice(),
        [Effect::ReadSession {
            request_id: 1,
            session: selected,
            branch: None,
        }] if *selected == session
    ));
    assert!(matches!(
        state.session,
        crate::inspector::LoadState::Loading { request_id: 1 }
    ));
}

/// Verifies activating a branch requests its activity subtree on demand.
#[test]
#[allow(clippy::field_reassign_with_default)]
fn session_branch_activation_requests_scoped_history() {
    let session = scriberig_core::EntityId::from_bytes([5; 16]);
    let branch = scriberig_core::EntityId::from_bytes([6; 16]);
    let mut state = TuiState::default();
    state.selection = Some(crate::selection::EntitySelection {
        entity: session,
        revision: None,
        snapshot: None,
    });
    state.primary_view = crate::selection::PrimaryView::Session;
    state.session = crate::inspector::LoadState::Ready(crate::view::session::SessionModel {
        rows: vec![crate::view::session::SessionRow {
            reference: branch,
            depth: 1,
            label: "branch".to_owned(),
            kind: crate::view::session::SessionRowKind::Branch,
            transient: true,
        }],
    });
    state.cursor = 0;
    let effects = activate_current(&mut state);
    assert!(matches!(
        effects.as_slice(),
        [Effect::ReadSession {
            session: selected,
            branch: Some(selected_branch),
            ..
        }] if *selected == session && *selected_branch == branch
    ));
}

/// Verifies durable result rows become ordinary semantic navigation intents.
#[test]
#[allow(clippy::field_reassign_with_default)]
fn session_result_activation_navigates_by_stable_reference() {
    let result = scriberig_core::EntityId::from_bytes([12; 16]);
    let mut state = TuiState::default();
    state.primary_view = crate::selection::PrimaryView::Session;
    state.session = crate::inspector::LoadState::Ready(crate::view::session::SessionModel {
        rows: vec![crate::view::session::SessionRow {
            reference: result,
            depth: 1,
            label: "result".to_owned(),
            kind: crate::view::session::SessionRowKind::Result,
            transient: false,
        }],
    });

    let effects = activate_current(&mut state);

    assert_eq!(effects, vec![Effect::None]);
    assert_eq!(
        state
            .navigation
            .current
            .and_then(|location| location.selection),
        Some(crate::selection::SemanticReference(result.to_string()))
    );
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

/// Verifies reconnect refreshes active agent operations and their sessions.
#[test]
fn reconnect_refreshes_active_agent_state() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("agent-operation").expect("operation");
    let session = scriberig_core::EntityId::from_bytes([10; 16]);
    let branch = scriberig_core::EntityId::from_bytes([11; 16]);
    state.connection = crate::state::ConnectionState::Disconnected;
    state.stale = true;
    state.agent_handles.push(crate::service::AgentHandle {
        operation: operation.clone(),
        session,
        branch: Some(branch),
    });

    let effects = reduce(&mut state, Action::ConnectionRestored);

    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::OperationStatus { operation: value } if value == &operation
    )));
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::ReadSession {
            session: value,
            branch: Some(selected),
            ..
        } if *value == session && *selected == branch
    )));
}
