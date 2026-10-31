//! Reduces exact-snapshot spreadsheet read requests and their results.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles spreadsheet read actions before the general TUI reducer.
///
/// # Parameters
/// - `state`: Presentation state receiving the request or response.
/// - `action`: Action to classify and reduce.
///
/// # Returns
/// Spreadsheet effects when the action belongs to this view, or `None` otherwise.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    if let Some(effects) = super::spreadsheet_cell::reduce(state, action) {
        return Some(effects);
    }
    match action {
        Action::SpreadsheetRequested {
            project,
            snapshot,
            worksheet,
            range,
            offset,
        } => {
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.primary_view = crate::selection::PrimaryView::Overview;
            state.document_tree = LoadState::NotRequested;
            state.artifact_page = LoadState::NotRequested;
            state.spreadsheet_page = LoadState::Loading { request_id };
            state.spreadsheet_cell = LoadState::NotRequested;
            state.spreadsheet_worksheet = Some(*worksheet);
            state.cursor = 0;
            Some(vec![Effect::InspectSpreadsheetPage {
                request_id,
                project: *project,
                snapshot: *snapshot,
                worksheet: *worksheet,
                range: *range,
                offset: *offset,
                limit: 40,
            }])
        }
        Action::SpreadsheetLoaded { request_id, page } => {
            if matches!(state.spreadsheet_page, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.connection = crate::state::ConnectionState::Connected;
                state.stale = false;
                state.spreadsheet_page = LoadState::Ready(page.clone());
            }
            Some(vec![Effect::None])
        }
        Action::SpreadsheetFailed {
            request_id,
            category,
            message,
        } => {
            if category == "transport_unavailable" {
                state.connection = crate::state::ConnectionState::Disconnected;
                state.stale = true;
            }
            if matches!(state.spreadsheet_page, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.spreadsheet_page = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            Some(vec![Effect::None])
        }
        _ => None,
    }
}
