//! Tests the module's deterministic behavior.

use super::{Focus, TuiState, reduce};
use crate::action::{Action, Direction};
use crate::effect::Effect;
use crate::event::{Key, KeyCode, Modifiers, TuiEvent};
use crate::overlay::{Overlay, palette::PaletteState};
use crate::selection::PrimaryView;
use crate::view::history::{HistoryModel, HistoryRow};
use scriberig_core::{EntityId, SnapshotId};

/// Verifies resize events update only terminal presentation state.
#[test]
fn resize_updates_state() {
    let mut state = TuiState::default();
    state.apply_event(&TuiEvent::Resize {
        width: 72,
        height: 24,
    });
    assert_eq!(state.terminal_size, (72, 24));
    assert!(!state.should_quit);
}

/// Verifies quit input stops the runtime.
#[test]
fn quit_key_sets_quit_flag() {
    let mut state = TuiState::default();
    state.apply_event(&TuiEvent::Key(Key {
        code: KeyCode::Char('q'),
        modifiers: Modifiers::default(),
    }));
    assert!(state.should_quit);
}

/// Verifies the control-C quit chord stops the runtime.
#[test]
fn control_c_sets_quit_flag() {
    let mut state = TuiState::default();
    state.apply_event(&TuiEvent::Key(Key {
        code: KeyCode::Char('c'),
        modifiers: Modifiers {
            control: true,
            ..Modifiers::default()
        },
    }));
    assert!(state.should_quit);
}

/// Verifies the global quit chord also works while Explorer owns focus.
#[test]
fn control_c_quits_from_explorer_focus() {
    let mut state = TuiState {
        focus: Focus::Explorer,
        ..TuiState::default()
    };
    state.apply_event(&TuiEvent::Key(Key {
        code: KeyCode::Char('\u{3}'),
        modifiers: Modifiers::default(),
    }));
    assert!(state.should_quit);
}

/// Verifies modal input takes precedence over browse commands.
#[test]
fn input_consumes_printable_keys() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::FindOpened);
    state.apply_event(&TuiEvent::Key(Key {
        code: KeyCode::Char('q'),
        modifiers: Modifiers::default(),
    }));
    assert_eq!(state.input_text(), Some("q"));
    assert!(!state.should_quit);
}

/// Verifies focus cycles without changing semantic history.
#[test]
fn focus_cycles_independently_of_navigation() {
    let mut state = TuiState::default();
    let history = state.navigation.clone();
    reduce(&mut state, Action::FocusNext);
    assert_eq!(state.focus, Focus::Inspector);
    reduce(&mut state, Action::FocusNext);
    assert_eq!(state.focus, Focus::Explorer);
    reduce(&mut state, Action::FocusNext);
    assert_eq!(state.focus, Focus::Primary);
    reduce(&mut state, Action::FocusPrevious);
    assert_eq!(state.focus, Focus::Explorer);
    assert_eq!(state.navigation, history);
}

/// Verifies overlays own input and close without changing navigation.
#[test]
fn overlay_closes_without_navigation_change() {
    let mut state = TuiState::default();
    let history = state.navigation.clone();
    reduce(&mut state, Action::HelpOpened);
    reduce(&mut state, Action::OverlayClosed);
    assert!(state.overlay.is_none());
    assert_eq!(state.navigation, history);
}

/// Verifies paste is inserted as one text payload.
#[test]
fn paste_is_input_data() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::FindOpened);
    reduce(&mut state, Action::Paste("q\n?".into()));
    assert_eq!(state.input_text(), Some("q\n?"));
    assert!(!state.should_quit);
}

/// Verifies pasted text is retained by the command palette.
#[test]
fn palette_paste_updates_filter_text() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::PaletteOpened);
    reduce(&mut state, Action::Paste("REF".into()));
    assert_eq!(
        state.overlay,
        Some(Overlay::Palette(PaletteState {
            query: "REF".to_owned(),
            ..PaletteState::default()
        }))
    );
}

/// Verifies palette cursor movement and activation stay local to the overlay.
#[test]
fn palette_selection_moves_and_activates() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::PaletteOpened);
    reduce(&mut state, Action::CursorMoved(Direction::Next));
    assert_eq!(
        state.overlay.as_ref().and_then(|overlay| match overlay {
            Overlay::Palette(palette) => Some(palette.selected),
            _ => None,
        }),
        Some(1)
    );
    let effects = reduce(&mut state, Action::Activated);
    assert_eq!(effects, vec![Effect::None]);
    assert!(state.overlay.is_none());
}

/// Verifies history palette activation closes the overlay before loading.
#[test]
fn palette_history_activation_closes_overlay() {
    let mut state = TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity: EntityId::from_bytes([8; 16]),
            revision: None,
            snapshot: Some(SnapshotId::new([3; 32])),
        }),
        ..TuiState::default()
    };
    reduce(&mut state, Action::PaletteOpened);
    for _ in 0..5 {
        reduce(&mut state, Action::CursorMoved(Direction::Next));
    }
    let effects = reduce(&mut state, Action::Activated);
    assert_eq!(
        effects,
        vec![Effect::History {
            request_id: 1,
            snapshot: SnapshotId::new([3; 32])
        }]
    );
    assert!(state.overlay.is_none());
    assert_eq!(state.primary_view, PrimaryView::History);
}

/// Verifies the palette reaches the agent composer for the selected entity.
#[test]
fn palette_opens_agent_composer_for_selection() {
    let entity = scriberig_core::EntityId::from_bytes([7; 16]);
    let mut state = TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity,
            revision: None,
            snapshot: None,
        }),
        ..TuiState::default()
    };
    reduce(&mut state, Action::PaletteOpened);
    reduce(&mut state, Action::InputChanged("start-agent".to_owned()));
    let effects = reduce(&mut state, Action::Activated);
    assert_eq!(effects, vec![Effect::None]);
    assert!(matches!(
        state.overlay,
        Some(Overlay::Composer(crate::overlay::ComposerState {
            session: selected,
            branch: None,
            ..
        })) if selected == entity
    ));
}

/// Verifies the palette can open the agent composer before a project exists.
#[test]
fn palette_opens_bootstrap_agent_composer_without_selection() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::PaletteOpened);
    reduce(&mut state, Action::InputChanged("start-agent".to_owned()));
    let effects = reduce(&mut state, Action::Activated);
    assert_eq!(effects, vec![Effect::None]);
    assert!(matches!(
        state.overlay,
        Some(Overlay::Composer(crate::overlay::ComposerState {
            session,
            branch: None,
            ..
        })) if session != EntityId::from_bytes([0; 16])
    ));
}

/// Verifies Diff collects an explicit target snapshot before dispatching.
#[test]
fn palette_diff_collects_target_snapshot() {
    let base = scriberig_core::SnapshotId::new([1; 32]);
    let target = scriberig_core::SnapshotId::new([2; 32]);
    let mut state = TuiState {
        selection: Some(crate::selection::EntitySelection {
            entity: scriberig_core::EntityId::from_bytes([7; 16]),
            revision: None,
            snapshot: Some(base),
        }),
        ..TuiState::default()
    };
    reduce(&mut state, Action::PaletteOpened);
    for _ in 0..6 {
        reduce(&mut state, Action::CursorMoved(Direction::Next));
    }
    reduce(&mut state, Action::Activated);
    reduce(&mut state, Action::Paste(target.to_string()));

    assert_eq!(
        reduce(&mut state, Action::InputSubmitted),
        vec![Effect::Diff {
            request_id: 1,
            request: scriberig_app::DiffRequest {
                from: base,
                to: target
            },
        }]
    );
    assert_eq!(state.primary_view, crate::selection::PrimaryView::Diff);
}

/// Verifies a history row activates its exact historical entity context.
#[test]
fn history_row_activation_inspects_historical_entity() {
    let entity = EntityId::from_bytes([9; 16]);
    let snapshot = SnapshotId::new([3; 32]);
    let mut state = TuiState {
        focus: Focus::Primary,
        primary_view: PrimaryView::History,
        history: crate::inspector::LoadState::Ready(HistoryModel {
            snapshot,
            rows: vec![HistoryRow {
                id: "snapshot:3".to_owned(),
                base: SnapshotId::new([2; 32]),
                result: snapshot,
                entities: vec![(entity, None)],
                workspace: None,
                source_change_id: None,
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
        }] if selected == &entity && selected_snapshot == &snapshot
    ));
}

/// Verifies a diff row activates inspection on the target snapshot side.
#[test]
fn diff_row_activation_inspects_target_entity() {
    let entity = EntityId::from_bytes([8; 16]);
    let target = SnapshotId::new([4; 32]);
    let mut state = TuiState {
        focus: Focus::Primary,
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
        focus: crate::state::Focus::Explorer,
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
    let effects = state.apply_event(&crate::event::TuiEvent::Application(
        scriberig_app::ApplicationEvent::OperationStarted {
            operation: operation.clone(),
        },
    ));
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
#[path = "state_tail_tests.rs"]
mod state_tail_tests;
