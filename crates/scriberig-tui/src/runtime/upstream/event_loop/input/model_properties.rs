//! Opens the upstream model property editor for the assigned model.

use crate::service::TuiService;

use super::{Focusable, InputContext, TUI};

/// Opens model properties using `ScribeRig`'s current assigned model selection.
///
/// - `context`: Borrowed terminal and application state.
///
/// Success after opening the upstream property editor or reporting its state.
/// # Returns
/// Completion of the property editor request.
///
/// # Errors
/// Returns an error when shared state locking fails.
pub(super) fn open<S: TuiService + Sync>(context: InputContext<'_, '_, S>) {
    let InputContext {
        service,
        context,
        state,
        screen,
        editor,
        input_sender,
        focus_before_overlay,
        redraw,
        ..
    } = context;
    let selection = state.lock().ok().and_then(|state| {
        if let crate::inspector::LoadState::Ready(session) = &state.session {
            Some((
                session.execution.provider_id.clone()?,
                session.execution.model_id.clone()?,
            ))
        } else {
            None
        }
    });
    let Some((provider_id, model_id)) = selection else {
        if let Ok(mut state) = state.lock() {
            "Select an assigned provider model before editing properties."
                .clone_into(&mut state.shell_message);
        }
        screen.request_render(false);
        return;
    };
    match service.model_properties(context, &provider_id, &model_id) {
        Ok(descriptors) => {
            let previous_focus = screen.get_focus();
            if super::super::super::property_editor::open_property_editor(
                screen,
                &provider_id,
                &model_id,
                descriptors,
                input_sender,
            ) {
                if focus_before_overlay.is_none() {
                    *focus_before_overlay = previous_focus;
                }
                editor.set_focused(false);
            } else if let Ok(mut state) = state.lock() {
                "The selected model has no editable upstream properties."
                    .clone_into(&mut state.shell_message);
            }
        }
        Err(_) => {
            if let Ok(mut state) = state.lock() {
                "Upstream model property settings are unavailable."
                    .clone_into(&mut state.shell_message);
            }
        }
    }
    *redraw = true;
}
