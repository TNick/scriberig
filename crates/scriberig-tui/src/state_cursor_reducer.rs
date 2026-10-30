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
    if matches!(state.overlay, Some(Overlay::Palette(_))) {
        // Palette selection and navigation are owned by the upstream selector.
        return Vec::new();
    }
    let last = cursor_limit(state);
    state.cursor = match direction {
        Direction::Previous => state.cursor.saturating_sub(1),
        Direction::Next => state.cursor.saturating_add(1).min(last),
    };
    if state.primary_view == crate::selection::PrimaryView::Session
        && let LoadState::Ready(model) = &state.session
    {
        state.session_cursor = model
            .rows
            .get(state.cursor)
            .map(crate::view::session::SessionRow::identity);
    }
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
    if matches!(
        state.primary_view,
        crate::selection::PrimaryView::Explorer
            | crate::selection::PrimaryView::History
            | crate::selection::PrimaryView::Find
            | crate::selection::PrimaryView::Diff
            | crate::selection::PrimaryView::Operations
    ) {
        crate::runtime::retain_semantic_row_cursor(state);
    }
    if state.primary_view == crate::selection::PrimaryView::Plan {
        crate::runtime::retain_plan_cursor(state);
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
    match state.primary_view {
        crate::selection::PrimaryView::Conversation => state
            .conversation
            .as_ref()
            .map_or(0, |model| model.rows.len().saturating_sub(1)),
        crate::selection::PrimaryView::Explorer => state
            .explorer
            .root
            .iter()
            .count()
            .saturating_add(state.explorer.rows.len())
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
        crate::selection::PrimaryView::Plan => {
            state.session_snapshot.as_ref().map_or(0, |session| {
                session
                    .plans
                    .iter()
                    .map(|plan| plan.nodes.len().saturating_add(1))
                    .sum::<usize>()
                    .min(200)
                    .saturating_sub(1)
            })
        }
        crate::selection::PrimaryView::Changes => match &state.review {
            LoadState::Ready(model) => model
                .conflicts
                .len()
                .saturating_add(model.details.len())
                .saturating_sub(1),
            _ => 0,
        },
        _ => 0,
    }
}
