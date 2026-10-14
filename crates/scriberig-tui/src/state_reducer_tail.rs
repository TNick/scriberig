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
        Action::CapabilitiesLoaded(capabilities) => Some(capabilities_loaded(state, capabilities)),
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
        Action::SessionSucceeded { request_id, model } => {
            if matches!(state.session, LoadState::Loading { request_id: current } if current == request_id)
            {
                if let Some(branch) = state.session_branch_focus.take()
                    && let Some(index) = model.rows.iter().position(|row| {
                        row.reference == branch
                            && row.kind == crate::view::session::SessionRowKind::Branch
                    })
                {
                    state.cursor = index;
                }
                state.session = LoadState::Ready(model);
            }
            Some(vec![Effect::None])
        }
        Action::AgentSubmitted(handle) => Some(agent_submitted(state, handle)),
        Action::SessionRequested { session, branch } => {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            state.agent_output_observations.clear();
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

/// Applies refreshed runtime capabilities to the shell state.
///
/// # Parameters
/// - `state`: TUI presentation receiving runtime capabilities.
/// - `capabilities`: Public capability descriptors from the application.
///
/// # Returns
/// A no-op effect after updating connection state.
fn capabilities_loaded(
    state: &mut TuiState,
    capabilities: Vec<scriberig_app::RuntimeCapability>,
) -> Vec<Effect> {
    state.capabilities = capabilities;
    state.connection = ConnectionState::Connected;
    state.stale = false;
    state.shell_message = "Semantic service attached.".to_owned();
    vec![Effect::None]
}

/// Updates the local presentation cache from an authoritative operation snapshot.
///
/// # Parameters
/// - `state`: TUI presentation receiving operation records.
/// - `records`: Current records supplied by the shared application service.
///
/// # Returns
/// A no-op effect after refreshing operation models.
fn operations_loaded(
    state: &mut TuiState,
    records: Vec<scriberig_app::OperationRecord>,
) -> Vec<Effect> {
    for record in records {
        let model = crate::view::operations::OperationModel::from_record(&record);
        if let Some(existing) = state
            .operations
            .iter_mut()
            .find(|existing| existing.id == model.id)
        {
            *existing = model;
        } else {
            state.operations.push(model);
        }
    }
    vec![Effect::None]
}

/// Refreshes the selected session after an agent operation reaches a terminal state.
///
/// # Parameters
/// - `state`: TUI state containing the operation and its accepted agent handle.
/// - `operation_id`: Exact operation whose terminal state was observed.
/// - `was_terminal`: Whether this operation had already reached a terminal state.
///
/// # Returns
/// A session read effect when the operation just completed for an agent handle.
fn refresh_terminal_agent_session(
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
/// # Parameters
/// - `state`: TUI state receiving the operation and workspace selection.
/// - `handle`: Accepted agent operation with optional workspace references.
///
/// # Returns
/// Effects required to refresh operation, session, and workspace views.
fn agent_submitted(state: &mut TuiState, handle: crate::service::AgentHandle) -> Vec<Effect> {
    let operation = handle.operation.clone();
    state
        .agent_handles
        .retain(|value| value.operation != operation);
    state.agent_handles.push(handle);
    let handle = state.agent_handles.last().cloned();
    let Some(handle) = handle else {
        return vec![Effect::OperationStatus { operation }];
    };
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
