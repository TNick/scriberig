//! Handles terminal agent operation refresh and submission.

use super::*;

/// Refreshes the selected session after an agent operation reaches a terminal state.
///
/// - `state`: TUI state containing the operation and its accepted agent handle.
/// - `operation_id`: Exact operation whose terminal state was observed.
/// - `was_terminal`: Whether this operation had already reached a terminal state.
///
/// A session read effect when the operation just completed for an agent handle.
/// # Returns
/// The operation result.
pub fn refresh_terminal_agent_session(
    state: &mut TuiState,
    operation_id: &scriberig_core::OperationId,
    was_terminal: bool,
) -> Vec<Effect> {
    let Some(operation_is_terminal) = state
        .operations
        .iter()
        .find(|operation| &operation.id == operation_id)
        .map(|operation| operation.status.is_terminal())
    else {
        return vec![Effect::None];
    };
    if was_terminal
        || !operation_is_terminal
        || state.primary_view != crate::selection::PrimaryView::Session
        || !matches!(state.session, LoadState::Ready(_))
    {
        return vec![Effect::None];
    }
    let Some(handle) = state
        .agent_handles
        .iter()
        .rev()
        .find(|handle| &handle.operation == operation_id)
    else {
        return vec![Effect::None];
    };
    let request_id = state.next_request_id;
    state.next_request_id += 1;
    state.session = LoadState::Loading { request_id };
    vec![Effect::ReadSession {
        request_id,
        session: handle.session,
        branch: handle.branch,
    }]
}

/// Records an accepted agent operation and activates its returned workspace.
///
/// - `state`: TUI state receiving the operation and workspace selection.
/// - `handle`: Accepted agent operation with optional workspace references.
///
/// Effects required to refresh operation, session, and workspace views.
/// # Returns
/// The operation result.
pub fn agent_submitted(state: &mut TuiState, handle: crate::service::AgentHandle) -> Vec<Effect> {
    let operation = handle.operation.clone();
    state
        .agent_handles
        .retain(|value| value.operation != operation);
    state.agent_handles.push(handle);
    let handle = state.agent_handles.last().cloned();
    let Some(handle) = handle else {
        return vec![Effect::OperationStatus { operation }];
    };
    if let Some(composer) = state.session_composer.as_mut() {
        composer.pending_prompt = None;
        composer.session = handle.session;
        composer.branch = handle.branch;
        composer.error = None;
    } else {
        state.session_composer = Some(crate::state::SessionComposerState {
            session: handle.session,
            branch: handle.branch,
            text: String::new(),
            pending_prompt: None,
            error: None,
        });
        state.session_composer_focused = true;
    }
    let workspace_effect = handle.workspace.as_ref().map(|workspace| {
        state.project = Some(workspace.name.clone());
        let selection = crate::selection::EntitySelection {
            entity: workspace.root,
            revision: None,
            snapshot: Some(workspace.snapshot),
        };
        state.selection = Some(selection);
        state.explorer = crate::explorer::ExplorerModel::from_selection(selection);
        let request_id = state.next_request_id;
        state.next_request_id += 1;
        state.inspector = LoadState::Loading { request_id };
        Effect::InspectEntity {
            request_id,
            entity: workspace.root,
            snapshot: Some(workspace.snapshot),
        }
    });
    state.primary_view = crate::selection::PrimaryView::Session;
    let request_id = state.next_request_id;
    state.next_request_id += 1;
    state.session = LoadState::Loading { request_id };
    let mut effects = vec![
        Effect::OperationStatus { operation },
        Effect::ReadSession {
            request_id,
            session: handle.session,
            branch: handle.branch,
        },
    ];
    effects.extend(workspace_effect);
    effects
}
