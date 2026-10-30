//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies diff row activation inspects target entity.
#[test]
fn diff_row_activation_inspects_target_entity() {
    let entity = EntityId::from_bytes([8; 16]);
    let target = SnapshotId::new([4; 32]);
    let mut state = TuiState {
        primary_view: PrimaryView::Diff,
        diff: crate::inspector::LoadState::Ready(crate::view::diff::DiffModel {
            left: SnapshotId::new([3; 32]),
            right: target,
            rows: vec![crate::view::diff::DiffRow {
                entity,
                before: None,
                after: None,
            }],
        }),
        ..TuiState::default()
    };

    let effects = reduce(&mut state, Action::Activated);

    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectEntity {
            entity: selected,
            snapshot: Some(selected_snapshot),
            ..
        }] if selected == &entity && selected_snapshot == &target
    ));
}

/// Verifies Enter on an Explorer row re-inspects that semantic entity.
#[test]
fn explorer_activation_inspects_selected_entity() {
    let entity = scriberig_core::EntityId::from_bytes([8; 16]);
    let mut state = TuiState {
        primary_view: crate::selection::PrimaryView::Explorer,
        ..TuiState::default()
    };
    state.explorer =
        crate::explorer::ExplorerModel::from_selection(crate::selection::EntitySelection {
            entity,
            revision: None,
            snapshot: None,
        });

    let effects = reduce(&mut state, Action::Activated);

    assert!(matches!(
        effects.as_slice(),
        [Effect::InspectEntity {
            entity: selected,
            ..
        }] if selected == &entity
    ));
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Overview);
}

/// Verifies refresh re-reads the selected entity through the service port.
#[test]
fn refresh_reissues_selected_inspection() {
    let mut state = TuiState::default();
    let entity = scriberig_core::EntityId::from_bytes([7; 16]);
    state.selection = Some(crate::selection::EntitySelection {
        entity,
        revision: None,
        snapshot: None,
    });
    let effects = reduce(&mut state, Action::RefreshRequested);
    assert!(
        matches!(effects.as_slice(), [Effect::InspectEntity { entity: selected, .. }] if selected == &entity)
    );
}

/// Verifies a start event exposes a cancellable operation before detail lookup completes.
#[test]
fn application_operation_event_immediately_exposes_running_state() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("running").expect("operation id");
    let effects =
        state.apply_application_event(&scriberig_app::ApplicationEvent::OperationStarted {
            operation: operation.clone(),
        });
    assert_eq!(state.operations[0].id, operation);
    assert_eq!(
        state.operations[0].status,
        scriberig_app::OperationStatus::Running
    );
    assert!(state.operations[0].cancellable());
    assert!(effects.contains(&Effect::OperationStatus { operation }));
}

/// Verifies closing the operations presentation cannot cancel application work.
#[test]
fn closing_overlay_does_not_cancel_operation() {
    let mut state = TuiState::default();
    let operation = scriberig_core::OperationId::new("active").expect("operation id");
    let record =
        scriberig_app::OperationRecord::queued(operation, scriberig_app::OperationKind::Query);
    reduce(
        &mut state,
        Action::OperationObserved(crate::view::operations::OperationModel::from_record(
            &record,
        )),
    );
    assert!(state.operations[0].cancellable());
    assert_eq!(
        reduce(&mut state, Action::OverlayClosed),
        vec![Effect::None]
    );
    assert!(state.operations[0].cancellable());
}

#[path = "state_late_tests.rs"]
mod state_late_tests;
#[path = "state_refresh_tests.rs"]
mod state_refresh_tests;
#[path = "state_session_composer_tests.rs"]
mod state_session_composer_tests;
#[path = "state_tail_tests.rs"]
mod state_tail_tests;
