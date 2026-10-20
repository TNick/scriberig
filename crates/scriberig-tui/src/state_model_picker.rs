//! Builds interactive model and thinking pickers from shared descriptors.

use crate::{
    effect::Effect,
    inspector::LoadState,
    overlay::{Overlay, palette::PaletteState},
    state::TuiState,
};
use scriberig_core::EntityId;

/// Opens a profile picker from the latest credential-safe model library.
///
/// - `state`: TUI state whose model picker should open.
/// - `library`: Current safe model and profile descriptors.
pub(crate) fn show_model_picker(
    state: &mut TuiState,
    library: &scriberig_protocol::ModelLibraryResponse,
) {
    let profiles = library
        .profiles
        .iter()
        .filter(|profile| profile.enabled)
        .map(|profile| {
            let model = library
                .models
                .iter()
                .find(|model| model.id == profile.model);
            let model_details = model.map_or_else(
                || profile.model.clone(),
                |model| {
                    format!(
                        "{} · {}/{}",
                        model.display_name, model.provider_profile, model.provider_model_id
                    )
                },
            );
            crate::overlay::palette::PaletteItem {
                id: format!("select-model:{}", profile.id),
                label: format!("{} · {model_details}", profile.display_name),
                source: crate::overlay::palette::ActionSource::Local,
            }
        })
        .collect::<Vec<_>>();
    if profiles.is_empty() {
        state.conversation_composer.error =
            Some("No enabled model profiles are available.".to_owned());
        state.overlay = None;
        return;
    }
    state.conversation_composer.error = None;
    state.overlay = Some(Overlay::Palette(PaletteState {
        query: String::new(),
        local_actions: profiles,
        semantic_actions: Vec::new(),
        selected: 0,
    }));
}

/// Opens a credential-safe model profile picker for one selected plan node.
///
/// - `state`: TUI state holding the exact plan assignment target.
/// - `library`: Shared credential-safe model library.
///
/// Unit after showing enabled profile and model options.
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
        .profiles
        .iter()
        .filter(|profile| profile.enabled)
        .map(|profile| crate::overlay::palette::PaletteItem {
            id: format!("assign-plan-profile:{}", profile.id),
            label: format!("{} · {}", profile.display_name, profile.model),
            source: crate::overlay::palette::ActionSource::Local,
        })
        .collect::<Vec<_>>();
    if local_actions.is_empty() {
        state.conversation_composer.error =
            Some("No enabled model profiles are available.".to_owned());
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
    let Some((session, revision, model_id)) = active_selection(state) else {
        "Load a session before selecting a thinking level.".clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    };
    let _ = (session, revision);
    let model_id = model_id.to_owned();
    if let LoadState::Ready(library) = &state.model_library {
        let levels = library
            .models
            .iter()
            .find(|model| model.id == model_id)
            .map(|model| {
                model
                    .thinking_levels
                    .iter()
                    .map(|level| crate::overlay::palette::PaletteItem {
                        id: format!("select-thinking:{}", level.id),
                        label: level.label.clone(),
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
    "Loading available model capabilities…".clone_into(&mut state.shell_message);
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
        "Loading available model profiles…".clone_into(&mut state.shell_message);
        return Some(vec![Effect::ReadModelLibrary { request_id }]);
    }
    if item_id == "thinking-picker" {
        return Some(show_thinking_picker(state));
    }
    if let Some(profile) = item_id.strip_prefix("select-model:") {
        return Some(select_model_profile(state, profile));
    }
    item_id
        .strip_prefix("select-thinking:")
        .map(|level| select_thinking_level(state, level))
}

/// Selects one enabled profile for future session invocations.
///
/// - `state`: TUI state containing the active session and model library.
/// - `profile_id`: Stable enabled profile identity.
///
/// The revision-checked shared model-selection effect.
/// # Returns
/// The operation result.
pub(crate) fn select_model_profile(state: &mut TuiState, profile_id: &str) -> Vec<Effect> {
    let Some((session, revision)) = active_session(state) else {
        "Load a session before selecting a model.".clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    };
    let enabled = matches!(&state.model_library, LoadState::Ready(library)
        if library.profiles.iter().any(|profile| profile.id == profile_id && profile.enabled));
    if !enabled {
        "That model profile is not available for selection.".clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    }
    state.overlay = None;
    vec![Effect::SetSessionModel(
        scriberig_app::SessionModelSelectionRequest {
            session,
            expected_revision: revision,
            model_profile: profile_id.to_owned(),
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
    let Some((session, revision, model_id)) = active_selection(state) else {
        "Load a session before selecting a thinking level.".clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    };
    let advertised = matches!(&state.model_library, LoadState::Ready(library)
        if library.models.iter().any(|model| model.id == model_id
            && model.thinking_levels.iter().any(|level| level.id == level_id)));
    if !advertised {
        "That thinking level is not advertised by the selected model."
            .clone_into(&mut state.shell_message);
        state.overlay = None;
        return vec![Effect::None];
    }
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
/// Session identity, mutation revision, and current model id when loaded.
/// # Returns
/// The operation result.
fn active_selection(state: &TuiState) -> Option<(EntityId, u64, &str)> {
    let (session, revision) = active_session(state)?;
    let LoadState::Ready(model) = &state.session else {
        return None;
    };
    Some((session, revision, model.execution.model.as_deref()?))
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
