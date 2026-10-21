//! Handles upstream theme selection and model property editor inputs.

use std::io;
use std::sync::Arc;

use rpi_tui::Component;
use rpi_tui::tui::{OverlayOptions, SizeValue};

use super::super::super::components::HelpPanel;
use super::{ComposerInput, Focusable, InputContext, TUI};

/// Applies one theme or model property editor event.
///
/// - `context`: Borrowed terminal and application state.
/// - `input`: Theme or model property event.
///
/// Completion after updating upstream presentation or property state.
/// # Returns
/// Completion of this terminal input event.
///
/// # Errors
/// Returns an error when shared state locking fails.
pub(super) fn handle<S>(context: InputContext<'_, '_, S>, input: ComposerInput) -> io::Result<()> {
    let InputContext {
        state,
        screen,
        editor,
        input_sender,
        focus_before_overlay,
        redraw,
        ..
    } = context;
    match input {
        ComposerInput::OpenThemeSelector => {
            let previous_focus = screen.get_focus();
            if super::super::super::selector_factory::open_theme_selector(screen, input_sender) {
                if focus_before_overlay.is_none() {
                    *focus_before_overlay = previous_focus;
                }
                editor.set_focused(false);
            }
            *redraw = true;
        }
        ComposerInput::BeginPropertyEdit {
            provider_id,
            model_id,
            key,
            current_value,
        } => {
            screen.hide_overlay();
            super::super::super::property_editor::open_property_value_editor(
                screen,
                &provider_id,
                &model_id,
                &key,
                &current_value,
                input_sender,
            );
            *redraw = true;
        }
        ComposerInput::OpenHelp => {
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
            crate::state::reduce(&mut state, crate::action::Action::HelpOpened);
            drop(state);
            if focus_before_overlay.is_none() {
                *focus_before_overlay = screen.get_focus();
            }
            let panel: Arc<dyn Component> = Arc::new(HelpPanel::new(input_sender.clone()));
            screen.show_overlay(
                Arc::clone(&panel),
                Some(OverlayOptions {
                    width: Some(SizeValue::Percent(0.72)),
                    max_height: Some(SizeValue::Percent(0.82)),
                    ..OverlayOptions::default()
                }),
            );
            screen.set_focus(Some(panel));
            editor.set_focused(false);
            *redraw = true;
        }
        ComposerInput::CloseHelp => {
            super::restore_overlay_focus(screen, editor, focus_before_overlay);
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
            crate::state::reduce(&mut state, crate::action::Action::OverlayClosed);
            *redraw = true;
        }
        ComposerInput::CancelSelection => {
            super::restore_overlay_focus(screen, editor, focus_before_overlay);
            if let Ok(mut state) = state.lock() {
                state.overlay = None;
            }
            *redraw = true;
        }
        _ => {}
    }
    Ok(())
}
