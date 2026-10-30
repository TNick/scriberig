//! Refreshes `ScribeRig` state after an upstream connection is restored.

use crate::{action::Action, effect::Effect, state::ConnectionState};

use super::super::TuiState;

/// Refreshes active semantic, operation, and session projections.
///
/// - `state`: TUI state receiving restored connection status.
///
/// # Returns
/// Refresh effects for the active selection and live agent handles.
#[must_use]
pub(super) fn reduce(state: &mut TuiState) -> Vec<Effect> {
    state.connection = ConnectionState::Connected;
    state.stale = true;
    let mut effects = if state.selection.is_some() {
        super::reduce(state, Action::RefreshRequested)
    } else {
        Vec::new()
    };
    for handle in state.agent_handles.clone() {
        if !state.pending_operation_refresh.contains(&handle.operation) {
            state
                .pending_operation_refresh
                .push(handle.operation.clone());
            effects.push(Effect::OperationStatus {
                operation: handle.operation,
            });
        }
        let request_id = state.next_request_id;
        state.next_request_id += 1;
        effects.push(Effect::ReadSession {
            request_id,
            session: handle.session,
            branch: handle.branch,
        });
    }
    if effects.is_empty() {
        effects.push(Effect::None);
    }
    effects
}
