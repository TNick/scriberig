//! Defines data-driven keyboard bindings and their presentation metadata.

use crate::{
    action::{Action, Direction},
    event::{Key, KeyCode, Modifiers},
    state::TuiState,
};

/// Interaction context used when resolving a key chord.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum Context {
    Browse,
    Input,
    Overlay,
}

/// Stable identifier for a local action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum ActionId {
    Quit,
    CloseOverlay,
    FocusNext,
    FocusPrevious,
    Activate,
    Back,
    Forward,
    Refresh,
    Help,
    Palette,
    Find,
    InputSubmit,
    CursorPrevious,
    CursorNext,
    CursorLeft,
    CursorRight,
}

/// One key binding with enough metadata for help and hints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub struct Binding {
    pub chord: KeyChord,
    pub context: Context,
    pub action: ActionId,
}

/// Backend-independent key chord.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub struct KeyChord {
    pub code: KeyCode,
    pub modifiers: Modifiers,
}

/// Human-readable metadata for an action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(missing_docs)]
pub struct ActionMetadata {
    pub id: ActionId,
    pub label: &'static str,
}

const NONE: Modifiers = Modifiers {
    control: false,
    alt: false,
    shift: false,
};
const SHIFT: Modifiers = Modifiers {
    control: false,
    alt: false,
    shift: true,
};
const CONTROL: Modifiers = Modifiers {
    control: true,
    alt: false,
    shift: false,
};

/// Returns the default local keymap.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn bindings() -> &'static [Binding] {
    &[
        Binding {
            chord: KeyChord {
                code: KeyCode::Char('c'),
                modifiers: CONTROL,
            },
            context: Context::Browse,
            action: ActionId::Quit,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Esc,
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::CloseOverlay,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Tab,
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::FocusNext,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Tab,
                modifiers: SHIFT,
            },
            context: Context::Browse,
            action: ActionId::FocusPrevious,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Enter,
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::Activate,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Up,
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::CursorPrevious,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Down,
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::CursorNext,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Left,
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::CursorLeft,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Right,
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::CursorRight,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Backspace,
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::Back,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Char(':'),
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::Palette,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Char('/'),
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::Find,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Help,
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::Help,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Char('r'),
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::Refresh,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Char('q'),
                modifiers: NONE,
            },
            context: Context::Browse,
            action: ActionId::Quit,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Esc,
                modifiers: NONE,
            },
            context: Context::Overlay,
            action: ActionId::CloseOverlay,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Help,
                modifiers: NONE,
            },
            context: Context::Overlay,
            action: ActionId::Help,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Char('c'),
                modifiers: CONTROL,
            },
            context: Context::Overlay,
            action: ActionId::Quit,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Enter,
                modifiers: NONE,
            },
            context: Context::Input,
            action: ActionId::InputSubmit,
        },
        Binding {
            chord: KeyChord {
                code: KeyCode::Backspace,
                modifiers: NONE,
            },
            context: Context::Input,
            action: ActionId::CursorLeft,
        },
    ]
}

/// Resolves a terminal key into a typed action using overlay precedence.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn action_for(state: &TuiState, key: &Key) -> Option<Action> {
    if state.is_input_active() {
        if key.modifiers.control && key.code == KeyCode::Char('c') {
            return Some(Action::QuitRequested);
        }
        return match key.code {
            KeyCode::Esc => Some(Action::OverlayClosed),
            KeyCode::Enter => Some(Action::InputSubmitted),
            KeyCode::Backspace => Some(Action::InputChanged(String::new())),
            KeyCode::Char(character) if !key.modifiers.control && !key.modifiers.alt => {
                Some(Action::InputChanged(character.to_string()))
            }
            _ => None,
        };
    }
    if matches!(state.overlay, Some(crate::overlay::Overlay::Confirm(_))) {
        return match key.code {
            KeyCode::Enter => Some(Action::ConfirmReview),
            KeyCode::Esc => Some(Action::CancelReview),
            _ => None,
        };
    }
    if matches!(state.overlay, Some(crate::overlay::Overlay::Palette(_))) {
        if key.code == KeyCode::Backspace {
            return Some(Action::InputChanged(String::new()));
        }
        let palette_action = match key.code {
            KeyCode::Enter => Some(Action::Activated),
            KeyCode::Up | KeyCode::Left => Some(Action::CursorMoved(Direction::Previous)),
            KeyCode::Down | KeyCode::Right => Some(Action::CursorMoved(Direction::Next)),
            _ => None,
        };
        if palette_action.is_some() {
            return palette_action;
        }
        if let KeyCode::Char(character) = key.code {
            if !key.modifiers.control && !key.modifiers.alt {
                return Some(Action::InputChanged(character.to_string()));
            }
        }
    }
    let context = if state.overlay.is_some() {
        Context::Overlay
    } else {
        Context::Browse
    };
    bindings()
        .iter()
        .find(|binding| {
            binding.context == context
                && binding.chord.code == key.code
                && binding.chord.modifiers == key.modifiers
        })
        .map(|binding| action(binding.action))
}

/// Converts an action identifier into the reducer's typed action.
fn action(id: ActionId) -> Action {
    match id {
        ActionId::Quit => Action::QuitRequested,
        ActionId::CloseOverlay => Action::OverlayClosed,
        ActionId::FocusNext => Action::FocusNext,
        ActionId::FocusPrevious => Action::FocusPrevious,
        ActionId::Activate => Action::Activated,
        ActionId::Back => Action::Back,
        ActionId::Forward => Action::Forward,
        ActionId::Refresh => Action::RefreshRequested,
        ActionId::Help => Action::HelpOpened,
        ActionId::Palette => Action::PaletteOpened,
        ActionId::Find => Action::FindOpened,
        ActionId::InputSubmit => Action::InputSubmitted,
        ActionId::CursorPrevious | ActionId::CursorLeft => Action::CursorMoved(Direction::Previous),
        ActionId::CursorNext | ActionId::CursorRight => Action::CursorMoved(Direction::Next),
    }
}

/// Returns metadata used by help and contextual key hints.
#[must_use]
pub fn metadata() -> &'static [ActionMetadata] {
    &[
        ActionMetadata {
            id: ActionId::Quit,
            label: "quit",
        },
        ActionMetadata {
            id: ActionId::CloseOverlay,
            label: "close overlay",
        },
        ActionMetadata {
            id: ActionId::FocusNext,
            label: "focus next",
        },
        ActionMetadata {
            id: ActionId::FocusPrevious,
            label: "focus previous",
        },
        ActionMetadata {
            id: ActionId::Activate,
            label: "activate",
        },
        ActionMetadata {
            id: ActionId::Back,
            label: "back",
        },
        ActionMetadata {
            id: ActionId::Refresh,
            label: "refresh",
        },
        ActionMetadata {
            id: ActionId::Help,
            label: "help",
        },
        ActionMetadata {
            id: ActionId::Palette,
            label: "command palette",
        },
        ActionMetadata {
            id: ActionId::Find,
            label: "find",
        },
        ActionMetadata {
            id: ActionId::Forward,
            label: "forward",
        },
        ActionMetadata {
            id: ActionId::InputSubmit,
            label: "submit",
        },
        ActionMetadata {
            id: ActionId::CursorPrevious,
            label: "cursor previous",
        },
        ActionMetadata {
            id: ActionId::CursorNext,
            label: "cursor next",
        },
        ActionMetadata {
            id: ActionId::CursorLeft,
            label: "cursor left",
        },
        ActionMetadata {
            id: ActionId::CursorRight,
            label: "cursor right",
        },
    ]
}

/// Returns bindings relevant to the current state for contextual hints.
#[must_use]
pub fn hints(state: &TuiState) -> Vec<(&'static str, &'static str)> {
    let context = if state.is_input_active() {
        Context::Input
    } else if state.overlay.is_some() {
        Context::Overlay
    } else {
        Context::Browse
    };
    bindings()
        .iter()
        .filter(|binding| binding.context == context)
        .filter_map(|binding| {
            metadata()
                .iter()
                .find(|item| item.id == binding.action)
                .map(|item| (key_label(binding.chord), item.label))
        })
        .collect()
}

/// Formats a key for help and hints.
pub(crate) fn key_label(chord: KeyChord) -> &'static str {
    match (chord.code, chord.modifiers) {
        (KeyCode::Char('c'), CONTROL) => "Ctrl-C",
        (KeyCode::Esc, _) => "Esc",
        (KeyCode::Tab, SHIFT) => "Shift-Tab",
        (KeyCode::Tab, _) => "Tab",
        (KeyCode::Enter, _) => "Enter",
        (KeyCode::Backspace, _) => "Backspace",
        (KeyCode::Help, _) => "?",
        (KeyCode::Char(':'), _) => ":",
        (KeyCode::Char('/'), _) => "/",
        (KeyCode::Char('r'), _) => "r",
        (KeyCode::Char('q'), _) => "q",
        (KeyCode::Up, _) => "Up",
        (KeyCode::Down, _) => "Down",
        (KeyCode::Left, _) => "Left",
        (KeyCode::Right, _) => "Right",
        _ => "key",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{Key, KeyCode, Modifiers},
        overlay::Overlay,
        state::TuiState,
    };
    #[test]
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
    #[test]
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
}
