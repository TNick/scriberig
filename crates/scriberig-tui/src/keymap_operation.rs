//! Resolves cancellation shortcuts against active application operation state.

use crate::{action::Action, state::TuiState};

/// Builds a cancellation action when the caller can cancel operations.
///
/// # Parameters
/// - `state`: Current TUI state with capabilities and operation observations.
/// - `super`: Parent keyboard binding module.
///
/// # Returns
/// A cancellation action for the root agent operation or another active operation.
pub(super) fn cancel_active_operation(state: &TuiState) -> Option<Action> {
    if !state
        .capabilities
        .iter()
        .any(|capability| capability.available && capability.id.as_str() == "operation_cancel")
    {
        return None;
    }
    state
        .operations
        .iter()
        .find(|operation| operation.cancellable() && operation.kind == "Agent")
        .or_else(|| {
            state
                .operations
                .iter()
                .find(|operation| operation.cancellable())
        })
        .map(|operation| Action::OperationCancelRequested(operation.id.clone()))
}
