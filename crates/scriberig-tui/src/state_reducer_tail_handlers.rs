//! Applies TUI session, agent, and operation reducer handlers.

use super::*;

/// Applies a current session read and reveals its pending branch target.
///
/// # Parameters
/// - `state`: TUI state receiving the session projection.
/// - `request_id`: Request token returned with the projection.
/// - `model`: Loaded session projection.
///
/// # Returns
/// A no-op effect after applying or rejecting the response.
pub(super) fn session_succeeded(
    state: &mut TuiState,
    request_id: u64,
    model: crate::view::session::SessionModel,
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
    }
    vec![Effect::None]
}

/// Applies a loaded model library to the pending model or thinking picker.
///
/// # Parameters
/// - `state`: TUI state receiving the model library.
/// - `request_id`: Request token returned with the library.
/// - `library`: Credential-safe model descriptors.
///
/// # Returns
/// A no-op effect after applying or rejecting the response.
pub(super) fn model_library_loaded(
    state: &mut TuiState,
    request_id: u64,
    library: scriberig_protocol::ModelLibraryResponse,
) -> Vec<Effect> {
    if matches!(state.model_library, LoadState::Loading { request_id: current } if current == request_id)
    {
        state.model_library = LoadState::Ready(library.clone());
        state.shell_message = "Model library loaded.".to_owned();
        if state.thinking_picker_pending.is_some() {
            state.thinking_picker_pending = None;
            let _ = super::show_thinking_picker(state);
        } else {
            super::show_model_picker(state, library);
        }
    }
    vec![Effect::None]
}

/// Applies an updated model and thinking selection to the session header.
///
/// # Parameters
/// - `state`: TUI state containing the active session.
/// - `selection`: Authoritative session execution selection.
///
/// # Returns
/// A no-op effect after updating the local projection.
pub(super) fn session_selection_updated(
    state: &mut TuiState,
    selection: scriberig_session::SessionExecutionSelection,
) -> Vec<Effect> {
    if let LoadState::Ready(session) = &mut state.session {
        let previous_model = session.execution.model.clone();
        let next_model = selection.model.clone();
        let previous_thinking = session.execution.thinking_level.clone();
        let next_thinking = selection.thinking_level.clone();
        session.set_execution(selection);
        if previous_model != next_model && previous_thinking != next_thinking {
            state.shell_message = format!(
                "Model changed to {}; thinking level adjusted to {}.",
                next_model.as_deref().unwrap_or("unspecified"),
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

/// Retains one sanitized live output observation for the session view.
///
/// # Parameters
/// - `state`: TUI state retaining bounded transient output.
/// - `output`: Provider-neutral output observation.
///
/// # Returns
/// A no-op effect after updating the transient observation list.
pub(super) fn agent_output_observed(state: &mut TuiState, output: &str) -> Vec<Effect> {
    state
        .agent_output_observations
        .push(crate::inspector::sanitize(output));
    if state.agent_output_observations.len() > 32 {
        state.agent_output_observations.remove(0);
    }
    state.shell_message = "agent output observed (non-canonical)".to_owned();
    vec![Effect::None]
}

/// Starts an authoritative read for one selected session.
///
/// # Parameters
/// - `state`: TUI state receiving the session read.
/// - `session`: Stable session identity.
/// - `branch`: Optional branch scope for the read.
///
/// # Returns
/// A session-read effect for an existing branch, or no effect for a new session.
pub(super) fn session_requested(
    state: &mut TuiState,
    session: scriberig_core::EntityId,
    branch: Option<scriberig_core::EntityId>,
) -> Vec<Effect> {
    let request_id = state.next_request_id;
    state.next_request_id += 1;
    state.agent_output_observations.clear();
    state.session = LoadState::Loading { request_id };
    vec![Effect::ReadSession {
        request_id,
        session,
        branch,
    }]
}

/// Opens the modal composer for one session branch.
///
/// # Parameters
/// - `state`: TUI state receiving the composer overlay.
/// - `session`: Stable session identity.
/// - `branch`: Optional branch receiving the submission.
///
/// # Returns
/// A no-op effect after opening the composer.
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
/// # Parameters
/// - `state`: TUI state receiving the persistent composer.
/// - `session`: Stable session identity.
/// - `branch`: Optional branch receiving the submission.
///
/// # Returns
/// A session-read effect.
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
    state.agent_output_observations.clear();
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

/// Applies refreshed runtime capabilities to the shell state.
///
/// # Parameters
/// - `state`: TUI presentation receiving runtime capabilities.
/// - `capabilities`: Public capability descriptors from the application.
///
/// # Returns
/// A no-op effect after updating connection state.
pub(super) fn capabilities_loaded(
    state: &mut TuiState,
    capabilities: Vec<scriberig_app::RuntimeCapability>,
) -> Vec<Effect> {
    state.capabilities = capabilities;
    state.connection = ConnectionState::Connected;
    state.stale = false;
    state.shell_message = "Semantic service attached.".to_owned();
    vec![Effect::ReadPresentationPreferences]
}

/// Updates the local presentation cache from an authoritative operation snapshot.
///
/// # Parameters
/// - `state`: TUI presentation receiving operation records.
/// - `records`: Current records supplied by the shared application service.
///
/// # Returns
/// A no-op effect after refreshing operation models.
pub(super) fn operations_loaded(
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
pub(super) fn refresh_terminal_agent_session(
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
pub(super) fn agent_submitted(
    state: &mut TuiState,
    handle: crate::service::AgentHandle,
) -> Vec<Effect> {
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
