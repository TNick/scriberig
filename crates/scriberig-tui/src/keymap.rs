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
    CancelOperation,
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
///
/// # Returns
/// The value produced by the declaration.
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
/// - `state`: Argument named `state`.
/// - `key`: Argument named `key`.
///
/// # Returns
/// The value produced by the declaration.
pub fn action_for(state: &TuiState, key: &Key) -> Option<Action> {
    if is_quit_chord(key) {
        return Some(Action::QuitRequested);
    }
    if state.is_input_active() {
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
    if key.code == KeyCode::Char('x')
        && key.modifiers == NONE
        && state
            .capabilities
            .iter()
            .any(|capability| capability.available && capability.id.as_str() == "operation_cancel")
    {
        return state
            .operations
            .iter()
            .find(|operation| operation.cancellable())
            .map(|operation| Action::OperationCancelRequested(operation.id.clone()));
    }
    bindings()
        .iter()
        .find(|binding| {
            binding.context == context
                && binding.chord.code == key.code
                && binding.chord.modifiers == command_modifiers(key)
        })
        .map(|binding| action(binding.action))
}

/// Recognizes Ctrl+C across terminal event encodings.
///
/// Some Windows terminal paths deliver the control byte directly, while
/// others deliver `c` or `C` with the control modifier attached.
///
/// # Parameters
/// - `key`: Backend-independent key event.
///
/// # Returns
/// Whether the key represents the global quit chord.
fn is_quit_chord(key: &Key) -> bool {
    matches!(key.code, KeyCode::Char('\u{3}'))
        || (key.modifiers.control && matches!(key.code, KeyCode::Char('c' | 'C')))
}

/// Returns modifiers normalized for punctuation command bindings.
///
/// Punctuation such as `:` and `?` commonly requires Shift on a physical
/// keyboard, but Shift produces the character rather than changing the
/// command chord.
///
/// # Parameters
/// - `key`: Key being matched against the command map.
///
/// # Returns
/// Modifiers normalized for command matching.
fn command_modifiers(key: &Key) -> Modifiers {
    if key.modifiers.shift
        && !key.modifiers.control
        && !key.modifiers.alt
        && matches!(key.code, KeyCode::Char(':') | KeyCode::Help)
    {
        return Modifiers {
            shift: false,
            ..key.modifiers
        };
    }
    key.modifiers
}

/// Converts an action identifier into the reducer's typed action.
/// - `id`: Argument named `id`.
///
/// # Returns
/// The value produced by the declaration.
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
        ActionId::CancelOperation => Action::OperationCancelRequested(
            scriberig_core::OperationId::new("keymap").expect("static operation id is valid"),
        ),
    }
}

#[path = "keymap_metadata.rs"]
mod keymap_metadata;
pub use keymap_metadata::{hints, key_label, metadata};

#[cfg(test)]
#[path = "keymap_tests.rs"]
mod tests;
