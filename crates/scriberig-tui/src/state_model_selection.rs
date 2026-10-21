//! Builds model and thinking pickers from shared upstream-derived rows.

use crate::{effect::Effect, inspector::LoadState, state::TuiState};
use scriberig_core::EntityId;

/// Selects one model for future session invocations.
///
/// - `state`: TUI state containing the active session and model library.
/// - `model_id`: Stable model-library identity used to resolve upstream IDs.
///
/// The revision-checked shared model-selection effect.
/// # Returns
/// The operation result.
pub(crate) fn select_model(state: &mut TuiState, model_id: &str) -> Vec<Effect> {
    let Some((session, revision)) = active_session(state) else {
        "Load a session before selecting a model.".clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    };
    let model = match &state.model_library {
        LoadState::Ready(library) => library.models.iter().find(|model| model.id == model_id),
        _ => None,
    };
    let Some(model) = model else {
        "That model is not available for selection.".clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    };
    state.overlay = None;
    vec![Effect::SetSessionModel(
        scriberig_app::SessionModelSelectionRequest {
            session,
            expected_revision: revision,
            provider_id: model.provider_id.clone(),
            model_id: model.provider_model_id.clone(),
        },
    )]
}

/// Selects an advertised level for future invocations of the current model.
///
/// - `state`: TUI state containing the active session and model library.
/// - `level_id`: Stable thinking-level identity advertised by that model.
///
/// The revision-checked shared thinking-selection effect.
/// # Returns
/// The operation result.
pub(crate) fn select_thinking_level(state: &mut TuiState, level_id: &str) -> Vec<Effect> {
    let Some((session, revision, _, _)) = active_selection(state) else {
        "Load a session before selecting a thinking level.".clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    };
    state.overlay = None;
    vec![Effect::SetSessionThinkingLevel(
        scriberig_app::SessionThinkingSelectionRequest {
            session,
            expected_revision: revision,
            thinking_level: level_id.to_owned(),
        },
    )]
}

/// Returns the active session and its authoritative selection revision.
///
/// - `state`: TUI state to inspect.
///
/// Session identity, mutation revision, and upstream provider/model IDs.
/// # Returns
/// The operation result.
fn active_selection(state: &TuiState) -> Option<(EntityId, u64, &str, &str)> {
    let (session, revision) = active_session(state)?;
    let LoadState::Ready(model) = &state.session else {
        return None;
    };
    Some((
        session,
        revision,
        model.execution.provider_id.as_deref()?,
        model.execution.model_id.as_deref()?,
    ))
}

/// Returns the active session and its authoritative selection revision.
///
/// - `state`: TUI state to inspect.
///
/// Session identity and mutation revision when a session is loaded.
/// # Returns
/// The operation result.
fn active_session(state: &TuiState) -> Option<(EntityId, u64)> {
    let LoadState::Ready(session) = &state.session else {
        return None;
    };
    let root = session
        .rows
        .iter()
        .find(|row| row.kind == crate::view::session::SessionRowKind::Session)?;
    Some((root.reference, session.execution.revision))
}

#[cfg(test)]
#[path = "state_model_selection_tests.rs"]
mod tests;
