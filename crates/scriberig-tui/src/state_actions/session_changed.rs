//! Refreshes the TUI session view after a matching durable session update.

use super::super::{Action, Effect, TuiState, reduce};

/// Refreshes the active session branch when it matches the event.
///
/// # Parameters
/// - `state`: TUI presentation state.
/// - `session`: Session identity reported by the application.
///
/// # Returns
/// Effects for a matching refresh, or no effect for an unrelated session.
pub(super) fn refresh_active_session(
    state: &mut TuiState,
    session: scriberig_core::EntityId,
) -> Vec<Effect> {
    let Some(branch) = state
        .session_composer
        .as_ref()
        .filter(|composer| composer.session == session)
        .map(|composer| composer.branch)
    else {
        return vec![Effect::None];
    };
    reduce(state, Action::SessionRequested { session, branch })
}
