//! Reduces shared document paragraph candidate and review transitions.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles paragraph candidate requests and opens the common review workflow.
///
/// # Parameters
/// - `state`: TUI state receiving request and candidate lifecycles.
/// - `action`: Action to classify and reduce.
///
/// # Returns
/// Candidate effects when the action belongs to document edits.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::DocumentEditRequested(request) => {
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.document_edit = LoadState::Loading { request_id };
            state.primary_view = crate::selection::PrimaryView::Overview;
            Some(vec![Effect::ProposeDocumentParagraphEdit {
                request_id,
                request: request.clone(),
            }])
        }
        Action::DocumentTableCellEditRequested(request) => {
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.document_cell_edit = LoadState::Loading { request_id };
            state.primary_view = crate::selection::PrimaryView::Overview;
            Some(vec![Effect::ProposeDocumentTableCellEdit {
                request_id,
                request: request.clone(),
            }])
        }
        Action::DocumentEditProposed {
            request_id,
            candidate,
        } => {
            if matches!(state.document_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.document_edit = LoadState::Ready(candidate.clone());
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
        Action::DocumentCellEditProposed {
            request_id,
            candidate,
        } => {
            if matches!(state.document_cell_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.document_cell_edit = LoadState::Ready(candidate.clone());
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
        Action::DocumentCellEditFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.document_cell_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.document_cell_edit = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            Some(vec![Effect::None])
        }
        Action::DocumentEditFailed {
            request_id,
            category,
            message,
        } => {
            if matches!(state.document_edit, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.document_edit = LoadState::Failed {
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
