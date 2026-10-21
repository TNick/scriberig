//! Resolves keyboard helper actions for the TUI.

use super::{Action, ActionId, Direction, Key, KeyCode, Modifiers};

/// Recognizes Ctrl+C across terminal event encodings.
///
/// Some Windows terminal paths deliver the control byte directly, while
/// others deliver `c` or `C` with the control modifier attached.
///
/// - `key`: Backend-independent key event.
///
/// Whether the key represents the global quit chord.
/// # Returns
/// `true` when the key is the global quit chord.
pub(super) fn is_quit_chord(key: &Key) -> bool {
    matches!(key.code, KeyCode::Char('\u{3}'))
        || (key.modifiers.control && matches!(key.code, KeyCode::Char('c' | 'C')))
}

/// Returns modifiers normalized for punctuation command bindings.
///
/// Punctuation such as `:` and `?` commonly requires Shift on a physical
/// keyboard, but Shift produces the character rather than changing the
/// command chord.
///
/// - `key`: Key being matched against the command map.
///
/// Modifiers normalized for command matching.
/// # Returns
/// The operation result.
pub(super) fn command_modifiers(key: &Key) -> Modifiers {
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
/// The operation result.
pub(super) fn action(id: ActionId) -> Action {
    match id {
        ActionId::Quit => Action::QuitRequested,
        ActionId::CloseOverlay => Action::OverlayClosed,
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
