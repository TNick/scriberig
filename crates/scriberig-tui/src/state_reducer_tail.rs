//! Reduces terminal session and agent actions.

#![allow(clippy::wildcard_imports, clippy::assigning_clones)]

use super::*;

/// Handles the terminal reducer action variants.
/// - `super`: Parent-module imports used by this helper.
/// - `state`: TUI state to update.
/// - `action`: Terminal action to reduce.
///
/// # Returns
/// Effects when the action belongs to this reducer partition.
pub(super) fn reduce_tail(state: &mut TuiState, action: Action) -> Option<Vec<Effect>> {
    match action {
        Action::CapabilitiesLoaded(capabilities) => {
            state.capabilities = capabilities;
            state.connection = ConnectionState::Connected;
            state.stale = false;
            state.shell_message = "Semantic service attached.".to_owned();
            Some(vec![Effect::None])
        }
        Action::ExplorerSucceeded {
            request_id,
            rows,
            next,
        } => {
            state.explorer.rows.extend(rows);
            state.explorer.next = next;
            state.shell_message = format!(
                "Explorer loaded {} snapshot entities (request {request_id}).",
                state.explorer.rows.len()
            );
            Some(vec![Effect::None])
        }
        Action::SessionSucceeded { request_id, model } => {
            if matches!(state.session, LoadState::Loading { request_id: current } if current == request_id)
            {
                state.session = LoadState::Ready(model);
                state.agent_output_observations.clear();
            }
            Some(vec![Effect::None])
        }
        Action::AgentSubmitted(handle) => {
            let operation = handle.operation.clone();
            state
                .agent_handles
                .retain(|value| value.operation != operation);
            state.agent_handles.push(handle);
            let handle = state.agent_handles.last().cloned();
            let Some(handle) = handle else {
                return Some(vec![Effect::OperationStatus { operation }]);
            };
            state.primary_view = crate::selection::PrimaryView::Session;
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.session = LoadState::Loading { request_id };
            Some(vec![
                Effect::OperationStatus { operation },
                Effect::ReadSession {
                    request_id,
                    session: handle.session,
                    branch: handle.branch,
                },
            ])
        }
        Action::SessionRequested { session, branch } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.session = LoadState::Loading { request_id };
            Some(vec![Effect::ReadSession {
                request_id,
                session,
                branch,
            }])
        }
        Action::ComposerOpened { session, branch } => {
            state.overlay = Some(Overlay::Composer(crate::overlay::ComposerState {
                session,
                branch,
                text: String::new(),
                error: None,
            }));
            Some(vec![Effect::None])
        }
        Action::AgentOutputObserved(output) => {
            state
                .agent_output_observations
                .push(crate::inspector::sanitize(&output));
            if state.agent_output_observations.len() > 32 {
                state.agent_output_observations.remove(0);
            }
            state.shell_message = "agent output observed (non-canonical)".to_owned();
            Some(vec![Effect::None])
        }
        Action::OperationObserved(model) => {
            state.pending_operation_refresh.retain(|id| id != &model.id);
            if let Some(existing) = state.operations.iter_mut().find(|item| item.id == model.id) {
                *existing = model;
            } else {
                state.operations.push(model);
            }
            Some(vec![Effect::None])
        }
        Action::OperationStatusRequested(operation) => {
            Some(vec![Effect::OperationStatus { operation }])
        }
        Action::OperationCancelRequested(operation) => {
            Some(vec![Effect::CancelOperation { operation }])
        }
        _ => None,
    }
}
