//! Reduces primary semantic-view navigation actions.

use crate::{effect::Effect, inspector::LoadState, selection::PrimaryView, state::TuiState};

/// Selects one semantic view and starts any required view-specific refresh.
///
/// # Parameters
/// - `state`: Presentation state receiving the selected view.
/// - `view`: Primary semantic view to activate.
///
/// # Returns
/// Effects required to load the selected view's data.
pub(super) fn select(state: &mut TuiState, view: PrimaryView) -> Vec<Effect> {
    state.primary_view = view;
    if view == PrimaryView::Overview {
        state.artifact_page = LoadState::NotRequested;
        state.artifact_members = LoadState::NotRequested;
        state.spreadsheet_page = LoadState::NotRequested;
        state.spreadsheet_cell = LoadState::NotRequested;
    }
    state.semantic_row_cursor = None;
    if view == PrimaryView::Session {
        state.cursor = match (&state.session, state.session_cursor) {
            (LoadState::Ready(model), Some(identity)) => model.index_of(identity).unwrap_or(0),
            _ => 0,
        };
    } else {
        state.cursor = 0;
        state.semantic_row_cursor = None;
    }
    crate::runtime::reconcile_semantic_row_cursor(state);
    if view == PrimaryView::Plan {
        crate::runtime::reconcile_plan_cursor(state);
    }
    if view == PrimaryView::Operations {
        return vec![Effect::ListOperations];
    }
    if view == PrimaryView::Session {
        if let Some(selection) = state.selection {
            return super::reduce(
                state,
                crate::action::Action::SessionRequested {
                    session: selection.entity,
                    branch: None,
                },
            );
        }
        "Select a session entity to load it.".clone_into(&mut state.shell_message);
    }
    vec![Effect::None]
}
