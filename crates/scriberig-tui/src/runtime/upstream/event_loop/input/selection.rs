//! Applies a selected upstream provider, model, thinking, or theme choice.

use std::io;

use crate::service::TuiService;

use super::{
    ControlKind, Focusable, InputContext, SelectorChoice, TUI, open_control_selector,
    queue_effects, restore_overlay_focus,
};

/// Applies one selected upstream control result to `ScribeRig` state.
///
/// - `context`: Borrowed terminal and application state.
/// - `choice`: Stable selected control identity.
///
/// Success after updating the selected control or showing its follow-up selector.
/// # Returns
/// Completion status for the selector action.
///
/// # Errors
/// Returns an error when shared state locking or diagnostics fail.
pub(super) fn handle<S: TuiService + Sync>(
    context: InputContext<'_, '_, S>,
    choice: SelectorChoice,
) -> io::Result<()> {
    let InputContext {
        state,
        screen,
        editor,
        input_sender,
        work_sender,
        pending_effects,
        focus_before_overlay,
        shared_state,
        debug_log,
        ..
    } = context;
    let provider_choice = matches!(&choice, SelectorChoice::Provider(_));
    if provider_choice {
        screen.hide_overlay();
    } else {
        restore_overlay_focus(screen, editor, focus_before_overlay);
    }
    let mut state = state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
    state.overlay = None;
    match choice {
        SelectorChoice::Provider(provider_id) => {
            let library = match &state.model_library {
                crate::inspector::LoadState::Ready(library) => Some(library.clone()),
                _ => None,
            };
            drop(state);
            if let Some(library) = library {
                let previous_focus = screen.get_focus();
                let opened = open_control_selector(
                    screen,
                    shared_state,
                    ControlKind::Model,
                    &library,
                    Some(&provider_id),
                    input_sender,
                );
                if opened {
                    if focus_before_overlay.is_none() {
                        *focus_before_overlay = previous_focus;
                    }
                    editor.set_focused(false);
                } else {
                    restore_overlay_focus(screen, editor, focus_before_overlay);
                }
            } else {
                restore_overlay_focus(screen, editor, focus_before_overlay);
            }
        }
        SelectorChoice::Model(model_id) => {
            let effects = crate::state::select_model(&mut state, &model_id);
            queue_effects(work_sender, pending_effects, effects);
            if let Some(log) = debug_log.as_deref_mut() {
                log.record_message("model-selected", &state)?;
            }
        }
        SelectorChoice::PlanModel(model_id) => {
            let item_id = format!("assign-plan-model:{model_id}");
            if let Some(effects) = crate::state::activate_palette_control(&mut state, &item_id) {
                queue_effects(work_sender, pending_effects, effects);
            }
            if let Some(log) = debug_log.as_deref_mut() {
                log.record_message("plan-model-assigned", &state)?;
            }
        }
        SelectorChoice::Thinking(level_id) => {
            let effects = match &state.model_library {
                crate::inspector::LoadState::Ready(library)
                    if current_model_advertises_thinking_level(&state, library, &level_id) =>
                {
                    crate::state::select_thinking_level(&mut state, &level_id)
                }
                _ => {
                    state.overlay = None;
                    "That thinking level is no longer available for the selected model."
                        .clone_into(&mut state.shell_message);
                    Vec::new()
                }
            };
            queue_effects(work_sender, pending_effects, effects);
            if let Some(log) = debug_log.as_deref_mut() {
                log.record_message("thinking-selected", &state)?;
            }
        }
        SelectorChoice::Theme(theme_id) => match rpi_tui::ThemePreset::from_id(&theme_id) {
            Some(preset) => rpi_tui::apply_theme_preset(preset),
            None => {
                "Upstream returned an unknown theme preset.".clone_into(&mut state.shell_message);
            }
        },
    }
    Ok(())
}

/// Checks a selected thinking ID against the currently assigned upstream model.
///
/// - `state`: Current `ScribeRig` session state.
/// - `library`: Credential-safe projection of the upstream model catalog.
/// - `level_id`: Selected scalar thinking ID.
///
/// Whether the current provider/model pair advertises the selected level.
/// # Returns
/// `true` only when the selection remains valid for the active model.
fn current_model_advertises_thinking_level(
    state: &crate::state::TuiState,
    library: &scriberig_protocol::ModelLibraryResponse,
    level_id: &str,
) -> bool {
    let crate::inspector::LoadState::Ready(session) = &state.session else {
        return false;
    };
    let Some(provider_id) = session.execution.provider_id.as_deref() else {
        return false;
    };
    let Some(model_id) = session.execution.model_id.as_deref() else {
        return false;
    };
    library.models.iter().any(|model| {
        model.provider_id == provider_id
            && model.provider_model_id == model_id
            && model.thinking_levels.iter().any(|level| level == level_id)
    })
}
