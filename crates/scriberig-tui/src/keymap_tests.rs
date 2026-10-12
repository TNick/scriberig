//! Tests the module's deterministic behavior.

use super::{Action, Direction, action_for, bindings, hints, metadata};
use crate::{
    event::{Key, KeyCode, Modifiers},
    overlay::Overlay,
    state::TuiState,
};
#[test]
/// Implements `printable_input_is_not_a_browse_command`.
fn printable_input_is_not_a_browse_command() {
    let state = TuiState {
        overlay: Some(Overlay::Input(crate::overlay::input::InputState::default())),
        ..TuiState::default()
    };
    assert_eq!(
        action_for(
            &state,
            &Key {
                code: KeyCode::Char('q'),
                modifiers: Modifiers::default()
            }
        ),
        Some(Action::InputChanged("q".into()))
    );
}

/// Verifies the shifted colon key opens the command palette.
#[test]
fn shifted_colon_opens_palette() {
    let state = TuiState::default();
    assert_eq!(
        action_for(
            &state,
            &Key {
                code: KeyCode::Char(':'),
                modifiers: Modifiers {
                    shift: true,
                    ..Modifiers::default()
                },
            }
        ),
        Some(Action::PaletteOpened)
    );
}

#[test]
/// Implements `hints_are_derived_from_bindings`.
fn hints_are_derived_from_bindings() {
    let state = TuiState::default();
    assert!(hints(&state).iter().any(|(_, label)| *label == "help"));
}

/// Verifies palette arrows and Enter resolve to typed local actions.
#[test]
fn palette_keys_control_selection_and_activation() {
    let state = TuiState {
        overlay: Some(Overlay::Palette(
            crate::overlay::palette::PaletteState::default(),
        )),
        ..TuiState::default()
    };
    assert_eq!(
        action_for(
            &state,
            &Key {
                code: KeyCode::Down,
                modifiers: Modifiers::default(),
            }
        ),
        Some(Action::CursorMoved(Direction::Next))
    );
    assert_eq!(
        action_for(
            &state,
            &Key {
                code: KeyCode::Enter,
                modifiers: Modifiers::default(),
            }
        ),
        Some(Action::Activated)
    );
}

/// Verifies help metadata covers every action represented by a binding.
#[test]
fn help_metadata_covers_all_bound_actions() {
    for binding in bindings() {
        assert!(metadata().iter().any(|item| item.id == binding.action));
    }
}
