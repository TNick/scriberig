//! Applies TUI session, agent, and operation reducer handlers.

use super::*;

#[path = "state_reducer_tail_handlers/session_commands.rs"]
mod session_commands;
pub(super) use session_commands::{session_removed, session_switched, sessions_listed};
#[path = "state_reducer_tail_handlers/agent_output.rs"]
mod agent_output;
pub(super) use agent_output::agent_output_observed;

/// Applies a current session read and reveals its pending branch target.
///
/// - `state`: TUI state receiving the session projection.
/// - `request_id`: Request token returned with the projection.
/// - `model`: Loaded session projection.
///
/// A no-op effect after applying or rejecting the response.
/// # Parameters
/// - `conversation`: Input for this operation.
/// - `session_snapshot`: Input for this operation.
/// # Returns
/// The operation result.
pub(super) fn session_succeeded(
    state: &mut TuiState,
    request_id: u64,
    model: crate::view::session::SessionModel,
    conversation: crate::view::conversation::ConversationModel,
    session_snapshot: Option<scriberig_session::Session>,
) -> Vec<Effect> {
    if matches!(state.session, LoadState::Loading { request_id: current } if current == request_id)
    {
        if let Some(branch) = state.session_branch_focus.take()
            && let Some(index) = model.rows.iter().position(|row| {
                row.reference == branch && row.kind == crate::view::session::SessionRowKind::Branch
            })
        {
            state.cursor = index;
        }
        state.session = LoadState::Ready(model);
        state.session_snapshot = session_snapshot;
        if let Some(session) = state.session_snapshot.as_ref() {
            state
                .transient_agent_outputs
                .retain(|output| !super::super::output_is_durable(session, output));
        }
        state.conversation = Some(state.session_snapshot.as_ref().map_or_else(
            || conversation,
            |session| crate::state::conversation_projection(state, session),
        ));
        if state.follow_coordinator {
            state.coordinator_cursor = state
                .conversation
                .as_ref()
                .and_then(|model| model.coordinator_cursor);
        }
        if let Some(source) = state.conversation_cursor
            && let Some(index) = state
                .conversation
                .as_ref()
                .and_then(|model| model.index_of(source))
        {
            state.cursor = index;
        } else if let Some(row) = state
            .conversation
            .as_ref()
            .and_then(|model| model.rows.first())
        {
            state.conversation_cursor = Some(row.source);
            state.cursor = 0;
        }
        if state.primary_view == crate::selection::PrimaryView::Changes {
            return super::state_actions::review_latest_candidate(state);
        }
    }
    vec![Effect::None]
}

/// Applies a shared plan response to the cached durable session projection.
///
/// - `state`: TUI state receiving the authoritative plan.
/// - `plan`: Persisted plan returned by the shared application service.
///
/// A no-op effect after rebuilding the conversation projection.
/// # Returns
/// The operation result.
pub(super) fn plan_updated(state: &mut TuiState, mut plan: scriberig_session::Plan) -> Vec<Effect> {
    let Some(mut session) = state.session_snapshot.clone() else {
        state.conversation_composer.error = Some("Load a session before editing plans.".to_owned());
        return vec![Effect::None];
    };
    plan.session = Some(session.id);
    if let Some(existing) = session.plans.iter_mut().find(|item| item.id == plan.id) {
        *existing = plan.clone();
    } else {
        session.plans.push(plan);
    }
    state.conversation = Some(crate::state::conversation_projection(state, &session));
    state.coordinator_cursor = state
        .conversation
        .as_ref()
        .and_then(|model| model.coordinator_cursor);
    state.session_snapshot = Some(session);
    vec![Effect::None]
}

/// Applies accepted conversation work using only returned durable handles.
///
/// - `state`: TUI presentation receiving the accepted operation.
/// - `handle`: Application allocated operation, session, and branch.
///
/// A durable session refresh effect.
/// # Returns
/// The operation result.
pub(super) fn conversation_agent_accepted(
    state: &mut TuiState,
    handle: &crate::service::AgentHandle,
) -> Vec<Effect> {
    let Some(branch) = handle.branch else {
        state.conversation_composer.pending = false;
        if let Some(pending) = state.conversation_composer.pending_prompt.take() {
            state.conversation_composer.draft = pending;
            state
                .conversation_composer
                .editor
                .replace_text(&state.conversation_composer.draft);
        }
        state.conversation_composer.error = Some("accepted work has no branch handle".to_owned());
        return vec![Effect::None];
    };
    state.conversation_composer.target = crate::state::ConversationTarget::Existing {
        session: handle.session,
        reply_anchor: scriberig_session::ContinuationAnchor::BranchHead { branch },
    };
    state.conversation_composer.pending = false;
    state.conversation_composer.pending_prompt = None;
    state.conversation_composer.clear_draft();
    state.conversation_composer.error = handle
        .failure
        .as_ref()
        .map(|failure| failure.message.clone());
    state.conversation_browsing = false;
    state.conversation_cursor = Some(crate::view::conversation::ConversationSource::Branch(
        branch,
    ));
    state.agent_handles.push(handle.clone());
    state.session_branch_focus = Some(branch);
    let request_id = state.next_request_id;
    state.next_request_id = state.next_request_id.saturating_add(1);
    state.session = LoadState::Loading { request_id };
    vec![Effect::ReadSession {
        request_id,
        session: handle.session,
        branch: None,
    }]
}

/// Pins the selected replyable source without changing coordinator ownership.
///
/// - `state`: TUI state with a semantic navigation cursor.
///
/// A no-op effect after updating local reply presentation state.
/// # Returns
/// The operation result.
pub(super) fn conversation_pin_cursor(state: &mut TuiState) -> Vec<Effect> {
    let selected = state
        .conversation
        .as_ref()
        .and_then(|model| model.rows.get(state.cursor));
    let Some((session, anchor)) = selected.and_then(|row| {
        let session =
            state
                .conversation
                .as_ref()?
                .rows
                .iter()
                .find_map(|item| match item.source {
                    crate::view::conversation::ConversationSource::Session(session) => {
                        Some(session)
                    }
                    _ => None,
                })?;
        Some((session, row.reply_anchor?))
    }) else {
        state.shell_message = "This conversation row cannot receive a reply.".to_owned();
        return vec![Effect::None];
    };
    state.conversation_composer.target = crate::state::ConversationTarget::Existing {
        session,
        reply_anchor: anchor,
    };
    state.follow_coordinator = false;
    state.conversation_browsing = false;
    state.shell_message = "Reply target pinned to the selected conversation node.".to_owned();
    vec![Effect::None]
}

/// Toggles local child-history expansion for the selected stable branch.
///
/// - `state`: TUI state with a selected conversation branch.
///
/// A no-op effect after rebuilding the presentation projection.
/// # Returns
/// The operation result.
pub(super) fn conversation_toggle_expansion(state: &mut TuiState) -> Vec<Effect> {
    let Some(crate::view::conversation::ConversationSource::Branch(branch)) =
        state.conversation_cursor
    else {
        return vec![Effect::None];
    };
    if !state.expanded_branches.remove(&branch) {
        state.expanded_branches.insert(branch);
    }
    if let Some(session) = &state.session_snapshot {
        state.conversation = Some(crate::state::conversation_projection(state, session));
    }
    if let Some(source) = state.conversation_cursor
        && let Some(index) = state
            .conversation
            .as_ref()
            .and_then(|model| model.index_of(source))
    {
        state.cursor = index;
    }
    vec![Effect::None]
}

/// Applies a loaded model library to the pending model or thinking picker.
///
/// - `state`: TUI state receiving the model library.
/// - `request_id`: Request token returned with the library.
/// - `library`: Credential-safe model descriptors.
///
/// A no-op effect after applying or rejecting the response.
/// # Returns
/// The operation result.
pub(super) fn model_library_loaded(
    state: &mut TuiState,
    request_id: u64,
    library: &scriberig_protocol::ModelLibraryResponse,
) -> Vec<Effect> {
    if matches!(state.model_library, LoadState::Loading { request_id: current } if current == request_id)
    {
        state.model_library = LoadState::Ready(library.clone());
        state.shell_message = "Model library loaded.".to_owned();
        if state.thinking_picker_pending.is_some() {
            state.thinking_picker_pending = None;
            let _ = super::show_thinking_picker(state);
        } else if state.plan_assignment_target.is_some() {
            super::state_model_picker::show_plan_assignment_picker(state, library);
        } else {
            super::show_model_picker(state, library);
        }
    }
    vec![Effect::None]
}

/// Applies an updated model and thinking selection to the session header.
///
/// - `state`: TUI state containing the active session.
/// - `selection`: Authoritative session execution selection.
///
/// A no-op effect after updating the local projection.
/// # Returns
/// The operation result.
pub(super) fn session_selection_updated(
    state: &mut TuiState,
    selection: scriberig_session::SessionExecutionSelection,
) -> Vec<Effect> {
    if let Some(session) = state.session_snapshot.as_mut() {
        session.execution = selection.clone();
    }
    if let LoadState::Ready(session) = &mut state.session {
        let previous_model = (
            session.execution.provider_id.clone(),
            session.execution.model_id.clone(),
        );
        let next_model = (selection.provider_id.clone(), selection.model_id.clone());
        let previous_thinking = session.execution.thinking_level.clone();
        let next_thinking = selection.thinking_level.clone();
        session.set_execution(selection);
        if previous_model != next_model && previous_thinking != next_thinking {
            state.shell_message = format!(
                "Model changed to {}; thinking level adjusted to {}.",
                next_model.1.as_deref().unwrap_or("unspecified"),
                next_thinking.as_deref().unwrap_or("unspecified"),
            );
        } else if previous_thinking != next_thinking {
            state.shell_message = match (previous_thinking, next_thinking) {
                (Some(previous), Some(next)) => {
                    format!("Thinking level changed from {previous} to {next}.")
                }
                (None, Some(next)) => format!("Thinking level set to {next}."),
                (Some(previous), None) => {
                    format!("Thinking level {previous} is no longer selected.")
                }
                (None, None) => "Thinking level selection cleared.".to_owned(),
            };
        } else {
            state.shell_message = "Future model selection updated.".to_owned();
        }
    } else {
        state.shell_message = "Future model selection updated.".to_owned();
    }
    vec![Effect::None]
}

/// Starts an authoritative read for one selected session.
///
/// - `state`: TUI state receiving the session read.
/// - `session`: Stable session identity.
/// - `branch`: Optional branch scope for the read.
///
/// A session-read effect for an existing branch, or no effect for a new session.
/// # Returns
/// The operation result.
pub(super) fn session_requested(
    state: &mut TuiState,
    session: scriberig_core::EntityId,
    branch: Option<scriberig_core::EntityId>,
) -> Vec<Effect> {
    let request_id = state.next_request_id;
    state.next_request_id += 1;
    state.transient_agent_outputs.clear();
    state.session = LoadState::Loading { request_id };
    vec![Effect::ReadSession {
        request_id,
        session,
        branch,
    }]
}

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
