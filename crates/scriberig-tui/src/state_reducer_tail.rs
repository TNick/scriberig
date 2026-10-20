//! Reduces terminal session and agent actions.

#![allow(clippy::wildcard_imports, clippy::assigning_clones)]

use super::*;

/// Handles the terminal reducer action variants.
/// - `super`: Parent-module imports used by this helper.
/// - `state`: TUI state to update.
/// - `action`: Terminal action to reduce.
///
/// Effects when the action belongs to this reducer partition.
/// # Returns
/// The operation result.
#[allow(clippy::too_many_lines)] // The action routing table is kept exhaustive in one match.
pub(super) fn reduce_tail(state: &mut TuiState, action: Action) -> Option<Vec<Effect>> {
    match action {
        Action::CapabilitiesLoaded(capabilities) => Some(capabilities_loaded(state, capabilities)),
        Action::PresentationPreferencesLoaded(response) => Some(
            super::state_presentation::apply_shared_preferences(state, response),
        ),
        Action::OperationsLoaded(records) => Some(operations_loaded(state, records)),
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
        Action::SessionSucceeded {
            request_id,
            model,
            conversation,
            session_snapshot,
        } => Some(session_succeeded(
            state,
            request_id,
            model,
            conversation,
            session_snapshot,
        )),
        Action::PlanUpdated(plan) => Some(plan_updated(state, plan)),
        Action::ModelLibraryLoaded {
            request_id,
            library,
        } => Some(model_library_loaded(state, request_id, &library)),
        Action::ToolResultDetailLoaded(detail) => {
            if let Ok(result) = detail.result.parse() {
                state.tool_result_details.insert(result, detail);
            }
            Some(vec![Effect::None])
        }
        Action::ModelRequestDetailLoaded(detail) => {
            if let Ok(invocation) = detail.invocation.parse() {
                state.model_request_details.insert(invocation, detail);
                state.shell_message = "Captured model request loaded.".to_owned();
            }
            Some(vec![Effect::None])
        }
        Action::SessionExecutionSelectionUpdated(selection) => {
            Some(session_selection_updated(state, selection))
        }
        Action::AgentSubmitted(handle) => Some(agent_submitted(state, handle)),
        Action::ConversationAgentAccepted(handle) => {
            Some(conversation_agent_accepted(state, &handle))
        }
        Action::ConversationBrowsing(browsing) => {
            state.conversation_browsing = browsing;
            Some(vec![Effect::None])
        }
        Action::ConversationPinCursor => Some(conversation_pin_cursor(state)),
        Action::ConversationFollowCoordinator => {
            state.follow_coordinator = true;
            if let Some(cursor) = state
                .conversation
                .as_ref()
                .and_then(|model| model.coordinator_cursor)
            {
                state.coordinator_cursor = Some(cursor);
            }
            Some(vec![Effect::None])
        }
        Action::ConversationToggleExpansion => Some(conversation_toggle_expansion(state)),
        Action::SessionRequested { session, branch } => {
            Some(session_requested(state, session, branch))
        }
        Action::SessionsListed(sessions) => Some(sessions_listed(state, &sessions)),
        Action::SessionSwitched(session) => Some(session_switched(state, session)),
        Action::SessionRemoved(session) => Some(session_removed(state, session)),
        Action::ComposerOpened { session, branch } => Some(composer_opened(state, session, branch)),
        Action::SessionComposerOpened { session, branch } => {
            Some(session_composer_opened(state, session, branch))
        }
        Action::SessionComposerFocused(focused) => {
            state.session_composer_focused = focused && state.session_composer.is_some();
            Some(vec![Effect::None])
        }
        Action::AgentOutputObserved {
            operation,
            text,
            complete,
        } => Some(agent_output_observed(state, &operation, &text, complete)),
        Action::OperationObserved(model) => {
            let operation_id = model.id.clone();
            let was_terminal = state
                .operations
                .iter()
                .find(|item| item.id == operation_id)
                .is_some_and(|item| item.status.is_terminal());
            state
                .pending_operation_refresh
                .retain(|id| id != &operation_id);
            if let Some(existing) = state
                .operations
                .iter_mut()
                .find(|item| item.id == operation_id)
            {
                *existing = model;
            } else {
                state.operations.push(model);
            }
            Some(refresh_terminal_agent_session(
                state,
                &operation_id,
                was_terminal,
            ))
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

#[path = "state_reducer_tail_handlers.rs"]
mod handlers;
use handlers::*;

/// Detaches the active presentation from saved history for a new conversation.
///
/// The durable session remains available through `/sessions`; the next user
/// message creates a separate session through the shared application API.
///
/// - `state`: TUI state whose active conversation is being cleared.
///
/// A no-op effect after returning the conversation home to its empty state.
/// # Returns
/// The operation result.
pub(super) fn new_conversation(state: &mut TuiState) -> Vec<Effect> {
    state.session = LoadState::NotRequested;
    state.session_snapshot = None;
    state.conversation = None;
    state.conversation_cursor = None;
    state.conversation_composer.target = ConversationTarget::NewSession;
    state.conversation_composer.pending = false;
    state.conversation_composer.pending_prompt = None;
    state.conversation_composer.pending_prompt = None;
    state.conversation_composer.error = None;
    state.conversation_browsing = false;
    state.coordinator_cursor = None;
    state.follow_coordinator = true;
    state.expanded_branches.clear();
    state.session_branch_focus = None;
    state.session_composer = None;
    state.session_composer_focused = false;
    state.cursor = 0;
    state.primary_view = crate::selection::PrimaryView::Conversation;
    state.focus = Focus::Primary;
    state.overlay = None;
    vec![Effect::None]
}
