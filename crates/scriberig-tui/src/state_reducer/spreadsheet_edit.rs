//! Reduces shared spreadsheet edit candidate requests and results.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles spreadsheet edit actions and opens the common candidate review on success.
///
/// # Parameters
/// - `state`: Presentation state receiving the request or candidate.
/// - `action`: Action to classify and reduce.
///
/// # Returns
/// Candidate effects or review effects when the action belongs to spreadsheet editing.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::SpreadsheetEditRequested(request) => {
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.primary_view = crate::selection::PrimaryView::Overview;
            state.artifact_page = LoadState::NotRequested;
            state.spreadsheet_page = LoadState::NotRequested;
            state.spreadsheet_cell = LoadState::NotRequested;
            state.spreadsheet_edit = LoadState::Loading { request_id };
            state.document_tree = LoadState::NotRequested;
            Some(vec![Effect::ProposeSpreadsheetEdit {
                request_id,
                request: request.clone(),
            }])
        }
        Action::SpreadsheetEditProposed {
            request_id,
            candidate,
        } => {
            if matches!(state.spreadsheet_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.spreadsheet_edit = LoadState::Ready(candidate.clone());
                state.primary_view = crate::selection::PrimaryView::Changes;
                Some(crate::state::state_actions::request_candidate_review(
                    state,
                    scriberig_app::ApplyRequest {
                        base_snapshot: candidate.base_snapshot,
                        branch: None,
                        change: candidate.id.clone(),
                    },
                ))
            } else {
                Some(vec![Effect::None])
            }
        }
        Action::SpreadsheetEditFailed {
            request_id,
            category,
            message,
        } => {
            if category == "transport_unavailable" {
                state.connection = crate::state::ConnectionState::Disconnected;
                state.stale = true;
            }
            if matches!(state.spreadsheet_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.spreadsheet_edit = LoadState::Failed {
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
