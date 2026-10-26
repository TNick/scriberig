//! Reduces cursor movement actions.

#![allow(clippy::wildcard_imports)]

use super::*;
use crate::action::Direction;

/// Reduces one cursor movement action.
/// - `super`: Parent-module imports used by this helper.
/// - `state`: TUI state to update.
/// - `direction`: Cursor movement direction.
///
/// Effects produced by the movement.
/// # Returns
/// The operation result.
pub(super) fn reduce_cursor(state: &mut TuiState, direction: Direction) -> Vec<Effect> {
    if let Some(Overlay::Palette(palette)) = state.overlay.as_mut() {
        let last = palette.filtered().len().saturating_sub(1);
        palette.selected = match direction {
            Direction::Previous => palette.selected.saturating_sub(1),
            Direction::Next => palette.selected.saturating_add(1).min(last),
        };
    } else {
        let last = cursor_limit(state);
        state.cursor = match direction {
            Direction::Previous => state.cursor.saturating_sub(1),
            Direction::Next => state.cursor.saturating_add(1).min(last),
        };
        if state.primary_view == crate::selection::PrimaryView::Conversation {
            state.conversation_cursor = state
                .conversation
                .as_ref()
                .and_then(|model| model.rows.get(state.cursor))
                .map(|row| row.source);
            if state.conversation_browsing {
                state.follow_coordinator = false;
            }
        }
    }
    vec![Effect::None]
}

/// Returns the last selectable row in the focused presentation.
///
/// - `state`: Current TUI presentation state.
///
/// The zero-based upper cursor bound for the active pane.
/// # Returns
/// The operation result.
fn cursor_limit(state: &TuiState) -> usize {
    match state.focus {
        Focus::Explorer => state
            .explorer
            .root
            .iter()
            .count()
            .saturating_add(state.explorer.rows.len())
            .saturating_sub(1),
        Focus::Primary => match state.primary_view {
            crate::selection::PrimaryView::Conversation => state
                .conversation
                .as_ref()
                .map_or(0, |model| model.rows.len().saturating_sub(1)),
            crate::selection::PrimaryView::Explorer => state
                .explorer
                .rows
                .len()
                .saturating_add(usize::from(state.explorer.root.is_some()))
                .saturating_sub(1),
            crate::selection::PrimaryView::History => match &state.history {
                LoadState::Ready(model) => model.rows.len().saturating_sub(1),
                _ => 0,
            },
            crate::selection::PrimaryView::Find => match &state.find {
                LoadState::Ready(model) => model.rows.len().saturating_sub(1),
                _ => 0,
            },
            crate::selection::PrimaryView::Diff => match &state.diff {
                LoadState::Ready(model) => model.rows.len().saturating_sub(1),
                _ => 0,
            },
            crate::selection::PrimaryView::Session => match &state.session {
                LoadState::Ready(model) => model.rows.len().saturating_sub(1),
                _ => 0,
            },
            crate::selection::PrimaryView::Operations => state.operations.len().saturating_sub(1),
            crate::selection::PrimaryView::Changes => match &state.review {
                LoadState::Ready(model) => model
                    .conflicts
                    .len()
                    .saturating_add(model.details.len())
                    .saturating_sub(1),
                _ => 0,
            },
            _ => 0,
        },
        Focus::Inspector => 0,
    }
}

/// Advances focus in the requested direction when no overlay is active.
/// - `super`: Parent-module imports used by this helper.
/// - `state`: TUI state to update.
/// - `reverse`: Whether to move backward.
///
/// No-op effect after updating focus.
/// # Returns
/// The operation result.
pub(super) fn reduce_focus(state: &mut TuiState, reverse: bool) -> Vec<Effect> {
    if state.overlay.is_none() {
        state.focus = state.focus.next(reverse);
    }
    vec![Effect::None]
}
