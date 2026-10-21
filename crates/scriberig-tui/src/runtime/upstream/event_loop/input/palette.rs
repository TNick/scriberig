//! Routes upstream palette choices into `ScribeRig` action reducers.

use std::io;

use rpi_tui::{Focusable, TUI};

use crate::runtime::queue_effects;

use super::super::super::selector_factory::open_palette_selector;
use super::super::super::{ComposerInput, ControlKind};
use super::super::focus::restore_overlay_focus;
use super::InputContext;

/// Opens the upstream searchable command selector.
///
/// # Parameters
/// - `context`: Shared TUI state and upstream components used by the selector.
///
/// # Returns
/// Success after opening the upstream selector or leaving it closed.
///
/// # Errors
/// Returns an error when shared presentation state is poisoned.
pub(super) fn open<S>(context: &mut InputContext<'_, '_, S>) -> io::Result<()> {
    let mut state = context
        .state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
    crate::state::reduce(&mut state, crate::action::Action::PaletteOpened);
    drop(state);
    let previous_focus = context.screen.get_focus();
    if open_palette_selector(context.screen, context.shared_state, context.input_sender) {
        if context.focus_before_overlay.is_none() {
            *context.focus_before_overlay = previous_focus;
        }
        context.editor.set_focused(false);
    }
    Ok(())
}

/// Applies a stable action selected from the upstream command palette.
///
/// # Parameters
/// - `context`: Shared state, components, and event channels for the active UI.
/// - `item_id`: Stable action or model-control identifier.
///
/// # Returns
/// `true` when the caller should skip the normal loop tail.
///
/// # Errors
/// Returns an error when shared presentation state is poisoned.
pub(super) fn select<S>(context: &mut InputContext<'_, '_, S>, item_id: &str) -> io::Result<bool> {
    restore_overlay_focus(context.screen, context.editor, context.focus_before_overlay);
    let routed_input = match item_id {
        "model-picker" => Some(ComposerInput::OpenControl(ControlKind::Model)),
        "thinking-picker" => Some(ComposerInput::OpenControl(ControlKind::Thinking)),
        "theme-picker" => Some(ComposerInput::OpenThemeSelector),
        _ => None,
    };
    if let Some(input) = routed_input {
        if let Ok(mut state) = context.state.lock() {
            state.overlay = None;
        }
        let _ = context.input_sender.send(input);
        return Ok(true);
    }
    let mut state = context
        .state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
    let effects = crate::state::activate_palette_item(&mut state, item_id);
    queue_effects(context.work_sender, context.pending_effects, effects);
    Ok(false)
}
