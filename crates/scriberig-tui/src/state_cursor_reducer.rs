//! Reduces cursor movement actions.

#![allow(clippy::wildcard_imports)]

use super::*;
use crate::action::Direction;

/// Reduces one cursor movement action.
/// - `super`: Parent-module imports used by this helper.
/// - `state`: TUI state to update.
/// - `direction`: Cursor movement direction.
///
/// # Returns
/// Effects produced by the movement.
pub(super) fn reduce_cursor(state: &mut TuiState, direction: Direction) -> Vec<Effect> {
    if let Some(Overlay::Palette(palette)) = state.overlay.as_mut() {
        let last = palette.filtered().len().saturating_sub(1);
        palette.selected = match direction {
            Direction::Previous => palette.selected.saturating_sub(1),
            Direction::Next => palette.selected.saturating_add(1).min(last),
        };
    } else {
        state.cursor = match direction {
            Direction::Previous => state.cursor.saturating_sub(1),
            Direction::Next => state.cursor.saturating_add(1),
        };
    }
    vec![Effect::None]
}

/// Advances focus in the requested direction when no overlay is active.
/// - `super`: Parent-module imports used by this helper.
/// - `state`: TUI state to update.
/// - `reverse`: Whether to move backward.
///
/// # Returns
/// No-op effect after updating focus.
pub(super) fn reduce_focus(state: &mut TuiState, reverse: bool) -> Vec<Effect> {
    if state.overlay.is_none() {
        state.focus = state.focus.next(reverse);
    }
    vec![Effect::None]
}
