//! Resolves focus and multiline input keys for the persistent composer.

use crate::{action::Action, event::Key, event::KeyCode, state::TuiState};

/// Resolves session-composer focus, blur, and newline shortcuts.
///
/// # Parameters
/// - `state`: Current TUI presentation and focus state.
/// - `key`: Backend-independent terminal key event.
///
/// # Returns
/// The composer action when the key belongs to this interaction.
pub(super) fn action_for(state: &TuiState, key: &Key) -> Option<Action> {
    if state.session_composer_focused && state.overlay.is_none() && key.code == KeyCode::Esc {
        return Some(Action::SessionComposerFocused(false));
    }
    if (state.session_composer_focused && state.overlay.is_none()
        || matches!(state.overlay, Some(crate::overlay::Overlay::Composer(_))))
        && key.code == KeyCode::Char('j')
        && key.modifiers.control
        && !key.modifiers.alt
        && !key.modifiers.shift
    {
        return Some(Action::InputChanged("\n".to_owned()));
    }
    if state.primary_view == crate::selection::PrimaryView::Session
        && state.overlay.is_none()
        && !state.session_composer_focused
        && state.session_composer.is_some()
        && key.code == KeyCode::Char('i')
        && key.modifiers == crate::event::Modifiers::default()
    {
        return Some(Action::SessionComposerFocused(true));
    }
    None
}
