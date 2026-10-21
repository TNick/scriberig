//! Tests the module's deterministic behavior.

use super::{TuiState, reduce};
use crate::action::{Action, Direction};
use crate::effect::Effect;
use crate::event::{Key, KeyCode, Modifiers, TuiEvent};
use crate::overlay::{Overlay, palette::PaletteState};
use crate::selection::PrimaryView;
use crate::view::history::{HistoryModel, HistoryRow};
use scriberig_core::{EntityId, SnapshotId};

/// Starts the native shell on the conversation home.
#[test]
fn default_state_opens_conversation_home() {
    assert_eq!(TuiState::default().primary_view, PrimaryView::Conversation);
}

/// Verifies ordinary conversation typing is not treated as a quit command.
#[test]
fn printable_q_edits_conversation_draft() {
    let mut state = TuiState::default();
    state.apply_event(&TuiEvent::Key(Key {
        code: KeyCode::Char('q'),
        modifiers: Modifiers::default(),
    }));
    assert!(!state.should_quit);
    assert_eq!(state.conversation_composer.draft, "q");
}

/// Verifies composer draft updates preserve the semantic reply target.
#[test]
fn conversation_draft_updates_without_changing_reply_target() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::InputChanged("a界b".to_owned()));
    reduce(&mut state, Action::InputChanged("x".to_owned()));

    assert_eq!(state.conversation_composer.draft, "a界bx");
    assert_eq!(
        state.conversation_composer.target,
        super::ConversationTarget::NewSession
    );
}

/// Verifies paste data is inserted without interpreting terminal controls.
#[test]
fn conversation_editor_sanitizes_pasted_control_sequences() {
    let mut state = TuiState::default();
    reduce(&mut state, Action::Paste("a\u{1b}[2J\nb".to_owned()));

    assert_eq!(state.conversation_composer.draft, "a\\u{1b}[2J\nb");
    assert!(!state.conversation_composer.draft.contains('\u{1b}'));
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

/// Verifies the global quit chord works independently of semantic view state.
#[test]
fn control_c_quits_from_the_active_view() {
    let mut state = TuiState::default();
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
        state.session_composer,
        Some(crate::state::SessionComposerState {
            session: selected,
            branch: None,
            ..
        }) if selected == entity
    ));
    assert!(state.overlay.is_none());
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
        state.session_composer,
        Some(crate::state::SessionComposerState {
            session,
            branch: None,
            ..
        }) if session != EntityId::from_bytes([0; 16])
    ));
    assert!(state.overlay.is_none());
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

#[path = "state_tests_part1.rs"]
mod part1;
