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
        SelectorChoice::Thinking(level_id) => {
            let effects = crate::state::select_thinking_level(&mut state, &level_id);
            queue_effects(work_sender, pending_effects, effects);
            if let Some(log) = debug_log.as_deref_mut() {
                log.record_message("thinking-selected", &state)?;
            }
        }
        SelectorChoice::Theme(theme_id) => match theme_id.as_str() {
            "dark" => rpi_tui::apply_theme_preset(rpi_tui::ThemePreset::Dark),
            "light" => rpi_tui::apply_theme_preset(rpi_tui::ThemePreset::Light),
            "monochrome" => rpi_tui::apply_theme_preset(rpi_tui::ThemePreset::Monochrome),
            _ => {
                "Upstream returned an unknown theme preset.".clone_into(&mut state.shell_message);
            }
        },
    }
    Ok(())
}
