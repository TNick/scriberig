//! Defines action metadata and contextual keyboard hints.

use super::{ActionId, ActionMetadata, Context, KeyChord, bindings};
use crate::event::{KeyCode, Modifiers};
use crate::state::TuiState;

/// Returns metadata used by help and contextual key hints.
///
/// Static action metadata.
#[must_use]
/// # Returns
/// The operation result.
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
        ActionMetadata {
            id: ActionId::CancelOperation,
            label: "cancel active operation",
        },
    ]
}

/// Returns bindings relevant to the current state for contextual hints.
///
/// - `state`: Current terminal state.
///
/// Keyboard hints for the active context.
#[must_use]
/// # Returns
/// The operation result.
pub fn hints(state: &TuiState) -> Vec<(&'static str, &'static str)> {
    let context = if state.is_input_active() {
        Context::Input
    } else if state.overlay.is_some() {
        Context::Overlay
    } else {
        Context::Browse
    };
    let mut hints = bindings()
        .iter()
        .filter(|binding| binding.context == context)
        .filter_map(|binding| {
            metadata()
                .iter()
                .find(|item| item.id == binding.action)
                .map(|item| (key_label(binding.chord), item.label))
        })
        .collect::<Vec<_>>();
    if !hints
        .iter()
        .any(|(key, label)| *key == "Ctrl-C" && *label == "quit")
    {
        hints.insert(0, ("Ctrl-C", "quit"));
    }
    hints
}

/// Formats a key for help and hints.
/// - `chord`: Key chord to format.
///
/// Human-readable key label.
#[must_use]
/// # Returns
/// The operation result.
pub fn key_label(chord: KeyChord) -> &'static str {
    match (chord.code, chord.modifiers) {
        (
            KeyCode::Char('c'),
            Modifiers {
                control: true,
                alt: false,
                shift: false,
            },
        ) => "Ctrl-C",
        (KeyCode::Esc, _) => "Esc",
        (
            KeyCode::Tab,
            Modifiers {
                control: false,
                alt: false,
                shift: true,
            },
        ) => "Shift-Tab",
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
