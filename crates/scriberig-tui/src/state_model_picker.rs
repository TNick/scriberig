//! Builds model and thinking pickers from shared upstream-derived rows.

use crate::{
    effect::Effect,
    inspector::LoadState,
    overlay::{Overlay, palette::PaletteState},
    state::TuiState,
};
use scriberig_core::EntityId;

/// Opens a model picker from the upstream-derived model library.
///
/// - `state`: TUI state whose model picker should open.
/// - `library`: Current safe upstream-derived model rows.
pub(crate) fn show_model_picker(
    state: &mut TuiState,
    library: &scriberig_protocol::ModelLibraryResponse,
) {
    let models = library
        .models
        .iter()
        .map(|model| crate::overlay::palette::PaletteItem {
            id: format!("select-model:{}", model.id),
            label: format!(
                "{} · {}/{}",
                model.display_name, model.provider_id, model.provider_model_id
            ),
            source: crate::overlay::palette::ActionSource::Local,
        })
        .collect::<Vec<_>>();
    if models.is_empty() {
        state.conversation_composer.error = Some("No models are available.".to_owned());
        state.overlay = None;
        return;
    }
    state.conversation_composer.error = None;
    state.overlay = Some(Overlay::Palette(PaletteState {
        query: String::new(),
        local_actions: models,
        semantic_actions: Vec::new(),
        selected: 0,
    }));
}

/// Opens a model picker for one selected plan node.
///
/// - `state`: TUI state holding the exact plan assignment target.
/// - `library`: Shared safe upstream-derived model library.
///
/// Unit after showing available model options.
pub(crate) fn show_plan_assignment_picker(
    state: &mut TuiState,
    library: &scriberig_protocol::ModelLibraryResponse,
) {
    if state.plan_assignment_target.is_none() {
        state.conversation_composer.error =
            Some("Select a plan node before assigning a model.".to_owned());
        return;
    }
    let local_actions = library
        .models
        .iter()
        .map(|model| crate::overlay::palette::PaletteItem {
            id: format!("assign-plan-model:{}", model.id),
            label: format!(
                "{} · {}/{}",
                model.display_name, model.provider_id, model.provider_model_id
            ),
            source: crate::overlay::palette::ActionSource::Local,
        })
        .collect::<Vec<_>>();
    if local_actions.is_empty() {
        state.conversation_composer.error = Some("No models are available.".to_owned());
        return;
    }
    state.overlay = Some(Overlay::Palette(PaletteState {
        query: String::new(),
        local_actions,
        semantic_actions: Vec::new(),
        selected: 0,
    }));
}

/// Opens the thinking picker for the currently selected model.
///
/// - `state`: TUI state whose thinking picker should open.
///
/// No effect when the picker opens, or a model-library read when needed.
/// # Returns
/// The operation result.
pub(crate) fn show_thinking_picker(state: &mut TuiState) -> Vec<Effect> {
    let Some((session, revision, provider_id, model_id)) = active_selection(state) else {
        "Load a session before selecting a thinking level.".clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    };
    let _ = (session, revision);
    let provider_id = provider_id.to_owned();
    let model_id = model_id.to_owned();
    if let LoadState::Ready(library) = &state.model_library {
        let levels = library
            .models
            .iter()
            .find(|model| model.provider_id == provider_id && model.provider_model_id == model_id)
            .map(|model| {
                model
                    .thinking_levels
                    .iter()
                    .map(|level| crate::overlay::palette::PaletteItem {
                        id: format!("select-thinking:{level}"),
                        label: level.clone(),
                        source: crate::overlay::palette::ActionSource::Local,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if levels.is_empty() {
            "The selected model has no advertised thinking levels."
                .clone_into(&mut state.shell_message);
            state.overlay = None;
        } else {
            state.overlay = Some(Overlay::Palette(PaletteState {
                query: String::new(),
                local_actions: levels,
                semantic_actions: Vec::new(),
                selected: 0,
            }));
        }
        return vec![Effect::None];
    }
    let request_id = state.next_request_id;
    state.next_request_id += 1;
    state.model_library = LoadState::Loading { request_id };
    state.thinking_picker_pending = Some(());
    state.overlay = None;
    "Loading available model options…".clone_into(&mut state.shell_message);
    vec![Effect::ReadModelLibrary { request_id }]
}

/// Routes one palette row to shared model or thinking controls.
///
/// - `state`: TUI state to update.
/// - `item_id`: Stable palette action identity.
///
/// Effects when the action belongs to model controls.
/// # Returns
/// The operation result.
pub(crate) fn activate_model_picker_action(
    state: &mut TuiState,
    item_id: &str,
) -> Option<Vec<Effect>> {
    if item_id == "model-picker" {
        state.overlay = None;
        state.thinking_picker_pending = None;
        if let LoadState::Ready(library) = state.model_library.clone() {
            show_model_picker(state, &library);
            return Some(vec![Effect::None]);
        }
        let request_id = state.next_request_id;
        state.next_request_id += 1;
        state.model_library = LoadState::Loading { request_id };
        state.conversation_composer.error = None;
        "Loading available models…".clone_into(&mut state.shell_message);
        return Some(vec![Effect::ReadModelLibrary { request_id }]);
    }
    if item_id == "thinking-picker" {
        return Some(show_thinking_picker(state));
    }
    if let Some(model) = item_id.strip_prefix("select-model:") {
        return Some(select_model(state, model));
    }
    item_id
        .strip_prefix("select-thinking:")
        .map(|level| select_thinking_level(state, level))
}

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
#[path = "state_model_picker_tests.rs"]
mod tests;
