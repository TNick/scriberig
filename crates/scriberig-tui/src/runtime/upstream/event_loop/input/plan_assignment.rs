//! Routes plan-model assignment commands to the upstream selector.

use std::io;

use rpi_tui::{Focusable, TUI};

use crate::action::Action;
use crate::runtime::queue_effects;

use super::super::super::ControlKind;
use super::super::super::selector_factory::open_control_selector;
use super::InputContext;

/// Opens the upstream model selector for the current plan-node assignment.
///
/// # Parameters
/// - `context`: Shared UI state and services used by assignment selection.
/// - `pending_selector`: Selector waiting for a model-library response.
/// - `prompt`: Submitted assignment command.
///
/// # Returns
/// Success after preparing the semantic target and opening or scheduling the
/// upstream selector.
///
/// # Errors
/// Returns an error when the shared presentation state lock is poisoned.
pub(super) fn open<S>(context: &mut InputContext<'_, '_, S>, prompt: &str) -> io::Result<()> {
    let mut state = context
        .state
        .lock()
        .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
    prompt.clone_into(&mut state.conversation_composer.draft);
    let effects = crate::state::reduce(&mut state, Action::InputSubmitted);
    queue_effects(context.work_sender, context.pending_effects, effects);
    let target_exists = state.plan_assignment_target.is_some();
    let library = match &state.model_library {
        crate::inspector::LoadState::Ready(library) => Some(library.clone()),
        _ => None,
    };
    drop(state);
    if !target_exists {
        return Ok(());
    }
    if let Some(library) = library {
        let previous_focus = context.screen.get_focus();
        if open_control_selector(
            context.screen,
            context.shared_state,
            ControlKind::PlanAssignment,
            &library,
            None,
            context.input_sender,
        ) {
            if context.focus_before_overlay.is_none() {
                *context.focus_before_overlay = previous_focus;
            }
            context.editor.set_focused(false);
        }
    } else {
        *context.pending_selector = Some(ControlKind::PlanAssignment);
    }
    Ok(())
}
