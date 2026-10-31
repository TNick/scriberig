//! Reduces bounded artifact inspection request and completion actions.

use crate::{action::Action, effect::Effect, inspector::LoadState, state::TuiState};

/// Handles artifact inspection actions before the general TUI action reducer.
///
/// # Parameters
/// - `state`: Presentation state receiving the artifact result.
/// - `action`: Action to classify and reduce.
///
/// # Returns
/// Effects when the action belongs to artifact inspection, or `None` otherwise.
pub(super) fn reduce(state: &mut TuiState, action: &Action) -> Option<Vec<Effect>> {
    match action {
        Action::ArtifactsRequested { project, offset } => {
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.primary_view = crate::selection::PrimaryView::Overview;
            state.document_tree = LoadState::NotRequested;
            state.artifact_page = LoadState::Loading { request_id };
            state.artifact_members = LoadState::NotRequested;
            state.spreadsheet_page = LoadState::NotRequested;
            state.spreadsheet_cell = LoadState::NotRequested;
            state.cursor = 0;
            Some(vec![Effect::InspectArtifactPage {
                request_id,
                project: *project,
                offset: *offset,
                limit: 20,
            }])
        }
        Action::ArtifactsLoaded { request_id, model } => {
            if matches!(state.artifact_page, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.connection = crate::state::ConnectionState::Connected;
                state.stale = false;
                state.artifact_page = LoadState::Ready(model.clone());
            }
            Some(vec![Effect::None])
        }
        Action::ArtifactsFailed {
            request_id,
            category,
            message,
        } => {
            if category == "transport_unavailable" {
                state.connection = crate::state::ConnectionState::Disconnected;
                state.stale = true;
            }
            if matches!(state.artifact_page, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.artifact_page = LoadState::Failed {
                    category: category.clone(),
                    message: message.clone(),
                    stale: false,
                };
            }
            Some(vec![Effect::None])
        }
        Action::ArtifactMembersRequested {
            project,
            artifact,
            offset,
        } => {
            let request_id = state.next_request_id;
            state.next_request_id = state.next_request_id.saturating_add(1);
            state.primary_view = crate::selection::PrimaryView::Overview;
            state.artifact_members = LoadState::Loading { request_id };
            Some(vec![Effect::InspectArtifactMembers {
                request_id,
                project: *project,
                artifact: *artifact,
                offset: *offset,
                limit: 20,
            }])
        }
        Action::ArtifactMembersLoaded { request_id, model } => {
            if matches!(state.artifact_members, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.connection = crate::state::ConnectionState::Connected;
                state.stale = false;
                state.artifact_members = LoadState::Ready(model.clone());
            }
            Some(vec![Effect::None])
        }
        Action::ArtifactMembersFailed {
            request_id,
            category,
            message,
        } => {
            if category == "transport_unavailable" {
                state.connection = crate::state::ConnectionState::Disconnected;
                state.stale = true;
            }
            if matches!(state.artifact_members, LoadState::Loading { request_id: current } if current == *request_id)
            {
                state.artifact_members = LoadState::Failed {
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
