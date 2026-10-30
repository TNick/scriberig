//! Reduces exact spreadsheet coordinate inspection actions.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles exact-cell inspection actions before the general TUI reducer.
///
/// # Parameters
/// - `state`: Presentation state receiving the request or response.
/// - `action`: Action to classify and reduce.
///
/// # Returns
/// Cell-inspection effects when applicable, or `None` for another action.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::SpreadsheetCellRequested(request) => {
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.primary_view = crate::selection::PrimaryView::Overview;
            state.artifact_page = LoadState::NotRequested;
            state.spreadsheet_page = LoadState::NotRequested;
            state.spreadsheet_cell = LoadState::Loading { request_id };
            state.document_tree = LoadState::NotRequested;
            state.spreadsheet_worksheet = Some(request.worksheet);
            state.cursor = 0;
            Some(vec![Effect::InspectSpreadsheetCell {
                request_id,
                request: *request,
            }])
        }
        Action::SpreadsheetCellLoaded { request_id, cell } => {
            if matches!(state.spreadsheet_cell, LoadState::Loading { request_id: current } if current == *request_id)
            {
                select_semantic_cell(state, cell);
                state.connection = crate::state::ConnectionState::Connected;
                state.stale = false;
                state.spreadsheet_cell = LoadState::Ready(cell.clone());
            }
            Some(vec![Effect::None])
        }
        Action::SpreadsheetCellFailed {
            request_id,
            category,
            message,
        } => {
            if category == "transport_unavailable" {
                state.connection = crate::state::ConnectionState::Disconnected;
                state.stale = true;
            }
            if matches!(state.spreadsheet_cell, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.spreadsheet_cell = LoadState::Failed {
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

/// Selects the exact cell entity when the inspected coordinate is nonblank.
///
/// # Parameters
/// - `state`: TUI state whose semantic selection is updated.
/// - `inspection`: Exact coordinate response from the application.
fn select_semantic_cell(
    state: &mut TuiState,
    inspection: &scriberig_protocol::SpreadsheetCellInspectResponse,
) {
    use std::str::FromStr as _;
    state.selection = inspection.cell.as_ref().and_then(|cell| {
        let entity = scriberig_core::EntityId::from_str(&cell.id).ok()?;
        let snapshot = scriberig_core::SnapshotId::from_str(&inspection.snapshot).ok()?;
        Some(crate::selection::EntitySelection {
            entity,
            revision: None,
            snapshot: Some(snapshot),
        })
    });
}
