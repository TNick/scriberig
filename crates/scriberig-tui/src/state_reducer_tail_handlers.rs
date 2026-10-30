//! Applies TUI session, agent, and operation reducer handlers.

use super::*;

#[path = "state_reducer_tail_handlers/session_projection.rs"]
mod session_projection;
pub(in crate::state::state_reducer_tail) use session_projection::{
    conversation_agent_accepted, conversation_pin_cursor, conversation_toggle_expansion,
    model_library_loaded, plan_updated, session_requested, session_selection_updated,
    session_succeeded,
};

#[path = "state_reducer_tail_handlers/session_commands.rs"]
mod session_commands;
pub(super) use session_commands::{session_removed, session_switched, sessions_listed};
#[path = "state_reducer_tail_handlers/agent_output.rs"]
mod agent_output;
pub(super) use agent_output::agent_output_observed;

/// Opens the modal composer for one session branch.
///
/// - `state`: TUI state receiving the composer overlay.
/// - `session`: Stable session identity.
/// - `branch`: Optional branch receiving the submission.
///
/// A no-op effect after opening the composer.
/// # Returns
/// The operation result.
pub(super) fn composer_opened(
    state: &mut TuiState,
    session: scriberig_core::EntityId,
    branch: Option<scriberig_core::EntityId>,
) -> Vec<Effect> {
    state.overlay = Some(Overlay::Composer(crate::overlay::ComposerState {
        session,
        branch,
        text: String::new(),
        error: None,
    }));
    vec![Effect::None]
}

/// Opens and focuses the persistent session composer while refreshing rows.
///
/// - `state`: TUI state receiving the persistent composer.
/// - `session`: Stable session identity.
/// - `branch`: Optional branch receiving the submission.
///
/// A session-read effect.
/// # Returns
/// The operation result.
pub(super) fn session_composer_opened(
    state: &mut TuiState,
    session: scriberig_core::EntityId,
    branch: Option<scriberig_core::EntityId>,
) -> Vec<Effect> {
    state.primary_view = crate::selection::PrimaryView::Session;
    state.overlay = None;
    let effects = if let Some(branch) = branch {
        let request_id = state.next_request_id;
        state.next_request_id += 1;
        state.session = LoadState::Loading { request_id };
        vec![Effect::ReadSession {
            request_id,
            session,
            branch: Some(branch),
        }]
    } else {
        state.session = LoadState::NotRequested;
        vec![Effect::None]
    };
    state.transient_agent_outputs.clear();
    state.session_composer = Some(crate::state::SessionComposerState {
        session,
        branch,
        text: String::new(),
        pending_prompt: None,
        error: None,
    });
    state.session_composer_focused = true;
    effects
}

#[path = "state_reducer_tail_handlers/capabilities.rs"]
mod capabilities;
pub(super) use capabilities::{capabilities_loaded, operations_loaded};

#[path = "state_reducer_tail_handlers/terminal.rs"]
mod terminal;
pub(super) use terminal::{agent_submitted, refresh_terminal_agent_session};
