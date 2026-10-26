//! Dispatches upstream composer input to `ScribeRig` semantic actions.

use std::io;
use std::sync::mpsc::TryRecvError;
use std::sync::{Arc, Mutex};

use rpi_tui::{Component, Editor, Focusable, TUI, TuiAltScreen};

use crate::action::Action;
use crate::debug_log::TuiDebugLog;
use crate::effect::Effect;
use crate::service::TuiService;
use crate::state::TuiState;

use super::super::super::{Work, queue_effects};
use super::super::selector_factory::open_control_selector;
use super::super::{ComposerInput, ControlKind, SelectorChoice};
use super::focus::restore_overlay_focus;
#[path = "input/context.rs"]
mod context;
#[path = "input/model_properties.rs"]
mod model_properties;
#[path = "input/palette.rs"]
mod palette;
#[path = "input/plan_assignment.rs"]
mod plan_assignment;
#[path = "input/selection.rs"]
mod selection;
#[path = "input/submit.rs"]
mod submit;
#[path = "input/theme_and_edit.rs"]
mod theme_and_edit;
pub(super) use context::InputContext;

macro_rules! input_context {
    (($($value:expr),* $(,)?)) => {{
        let (
            service, context, state, screen, editor, composer, semantic_view,
            input_sender, work_sender, pending_selector, pending_effects,
            focus_before_overlay, shared_state, debug_log, should_quit, redraw,
            suppress_composer_restore,
        ) = ($($value),*);
        InputContext {
            service, context, state, screen, editor, composer, semantic_view,
            input_sender, work_sender, pending_selector, pending_effects,
            focus_before_overlay, shared_state, debug_log, should_quit, redraw,
            suppress_composer_restore,
        }
    }};
}

/// Applies one upstream composer input and reports whether to skip the loop tail.
///
/// - `context`: Borrowed runtime state used by the selected input action.
/// - `input`: Received composer event or channel timeout/error.
///
/// `true` when input handling already requested a loop continuation.
/// # Returns
/// Whether the caller should continue before requesting a render.
///
/// # Errors
/// Returns an error when state locking or input diagnostics fail.
#[expect(
    clippy::too_many_lines,
    reason = "This is the exhaustive routing table for upstream composer events; selector and modal families use dedicated handlers."
)]
pub(super) fn handle<S: TuiService + Sync>(
    context: InputContext<'_, '_, S>,
    input: Result<ComposerInput, TryRecvError>,
) -> io::Result<bool> {
    let InputContext {
        service,
        context,
        state,
        screen,
        editor,
        composer,
        semantic_view,
        input_sender,
        work_sender,
        pending_selector,
        pending_effects,
        focus_before_overlay,
        shared_state,
        debug_log,
        should_quit,
        redraw,
        suppress_composer_restore,
    } = context;
    match input {
        Ok(ComposerInput::TogglePresentationFocus(focused)) => {
            let target: Arc<dyn Component> = if focused {
                composer.set_focused(false);
                Arc::clone(semantic_view) as Arc<dyn Component>
            } else {
                composer.set_focused(true);
                Arc::clone(composer) as Arc<dyn Component>
            };
            screen.set_focus(Some(target));
            if let Ok(mut state) = state.lock()
                && state.primary_view == crate::selection::PrimaryView::Conversation
            {
                let effects =
                    crate::state::reduce(&mut state, Action::ConversationBrowsing(focused));
                queue_effects(work_sender, pending_effects, effects);
            }
            *redraw = true;
        }
        Ok(ComposerInput::PresentationAction(action)) => {
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
            let action = if state.primary_view == crate::selection::PrimaryView::Conversation
                && state.conversation_browsing
                && *action == Action::Activated
            {
                Action::ConversationPinCursor
            } else {
                *action
            };
            let effects = crate::state::reduce(&mut state, action);
            queue_effects(work_sender, pending_effects, effects);
            *redraw = true;
        }
        Ok(ComposerInput::Submit(prompt)) => {
            return submit::handle(
                input_context!((
                    service,
                    context,
                    state,
                    screen,
                    editor,
                    composer,
                    semantic_view,
                    input_sender,
                    work_sender,
                    &mut *pending_selector,
                    &mut *pending_effects,
                    &mut *focus_before_overlay,
                    shared_state,
                    &mut *debug_log,
                    &mut *should_quit,
                    &mut *redraw,
                    &mut *suppress_composer_restore
                )),
                prompt,
            );
        }
        Ok(ComposerInput::DraftChanged(draft)) => {
            if !draft.is_empty() {
                *suppress_composer_restore = false;
            }
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
            state.conversation_composer.draft = draft;
            state.conversation_composer.error = None;
            *redraw = true;
        }
        Ok(ComposerInput::OpenControl(kind)) => {
            let library = {
                let mut state = state
                    .lock()
                    .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
                match &state.model_library {
                    crate::inspector::LoadState::Ready(library) => Some(library.clone()),
                    crate::inspector::LoadState::Loading { .. } => {
                        *pending_selector = Some(kind);
                        None
                    }
                    _ => {
                        let request_id = state.next_request_id;
                        state.next_request_id = state.next_request_id.saturating_add(1);
                        state.model_library = crate::inspector::LoadState::Loading { request_id };
                        *pending_selector = Some(kind);
                        queue_effects(
                            work_sender,
                            pending_effects,
                            vec![Effect::ReadModelLibrary { request_id }],
                        );
                        None
                    }
                }
            };
            if let Some(library) = library {
                let previous_focus = screen.get_focus();
                if open_control_selector(screen, state, kind, &library, None, input_sender) {
                    if focus_before_overlay.is_none() {
                        *focus_before_overlay = previous_focus;
                    }
                    editor.set_focused(false);
                }
            }
            *redraw = true;
        }
        Ok(ComposerInput::OpenModelProperties) => {
            model_properties::open(input_context!((
                service,
                context,
                state,
                screen,
                editor,
                composer,
                semantic_view,
                input_sender,
                work_sender,
                &mut *pending_selector,
                &mut *pending_effects,
                &mut *focus_before_overlay,
                shared_state,
                &mut *debug_log,
                &mut *should_quit,
                &mut *redraw,
                &mut *suppress_composer_restore
            )));
        }
        Ok(
            input @ (ComposerInput::OpenThemeSelector
            | ComposerInput::BeginPropertyEdit { .. }
            | ComposerInput::OpenHelp
            | ComposerInput::CloseHelp
            | ComposerInput::CancelSelection),
        ) => {
            theme_and_edit::handle(
                input_context!((
                    service,
                    context,
                    state,
                    screen,
                    editor,
                    composer,
                    semantic_view,
                    input_sender,
                    work_sender,
                    &mut *pending_selector,
                    &mut *pending_effects,
                    &mut *focus_before_overlay,
                    shared_state,
                    &mut *debug_log,
                    &mut *should_quit,
                    &mut *redraw,
                    &mut *suppress_composer_restore
                )),
                input,
            )?;
        }
        Ok(
            ComposerInput::ModelPropertyChanged {
                provider_id,
                model_id,
                key,
                value,
            }
            | ComposerInput::ModelPropertySubmitted {
                provider_id,
                model_id,
                key,
                value,
            },
        ) => {
            match service.set_model_property(context, &provider_id, &model_id, &key, value) {
                Ok(()) => {
                    if let Ok(mut state) = state.lock() {
                        "Model property updated for this process."
                            .clone_into(&mut state.shell_message);
                    }
                }
                Err(_) => {
                    if let Ok(mut state) = state.lock() {
                        "Upstream rejected the model property value."
                            .clone_into(&mut state.shell_message);
                    }
                }
            }
            restore_overlay_focus(screen, editor, focus_before_overlay);
            *redraw = true;
        }
        Ok(ComposerInput::CancelPropertySettings | ComposerInput::CancelPropertyEdit) => {
            restore_overlay_focus(screen, editor, focus_before_overlay);
            *redraw = true;
        }
        Ok(ComposerInput::OpenPalette) => {
            palette::open(&mut input_context!((
                service,
                context,
                state,
                screen,
                editor,
                composer,
                semantic_view,
                input_sender,
                work_sender,
                &mut *pending_selector,
                &mut *pending_effects,
                &mut *focus_before_overlay,
                shared_state,
                &mut *debug_log,
                &mut *should_quit,
                &mut *redraw,
                &mut *suppress_composer_restore
            )))?;
            *redraw = true;
        }
        Ok(ComposerInput::PaletteSelected(item_id)) => {
            if palette::select(
                &mut input_context!((
                    service,
                    context,
                    state,
                    screen,
                    editor,
                    composer,
                    semantic_view,
                    input_sender,
                    work_sender,
                    &mut *pending_selector,
                    &mut *pending_effects,
                    &mut *focus_before_overlay,
                    shared_state,
                    &mut *debug_log,
                    &mut *should_quit,
                    &mut *redraw,
                    &mut *suppress_composer_restore
                )),
                &item_id,
            )? {
                return Ok(true);
            }
            *redraw = true;
        }
        Ok(ComposerInput::PluginCommandAction {
            command,
            arguments,
            action,
        }) => {
            screen.hide_overlay();
            let response = service.invoke_plugin_command_action(&command, &arguments, action);
            palette::present_plugin_response(
                &mut input_context!((
                    service,
                    context,
                    state,
                    screen,
                    editor,
                    composer,
                    semantic_view,
                    input_sender,
                    work_sender,
                    &mut *pending_selector,
                    &mut *pending_effects,
                    &mut *focus_before_overlay,
                    shared_state,
                    &mut *debug_log,
                    &mut *should_quit,
                    &mut *redraw,
                    &mut *suppress_composer_restore
                )),
                &command,
                &arguments,
                response,
            )?;
            *redraw = true;
        }
        Ok(ComposerInput::Selected(choice)) => {
            selection::handle(
                input_context!((
                    service,
                    context,
                    state,
                    screen,
                    editor,
                    composer,
                    semantic_view,
                    input_sender,
                    work_sender,
                    &mut *pending_selector,
                    &mut *pending_effects,
                    &mut *focus_before_overlay,
                    shared_state,
                    &mut *debug_log,
                    &mut *should_quit,
                    &mut *redraw,
                    &mut *suppress_composer_restore
                )),
                choice,
            )?;
            *redraw = true;
        }
        Ok(
            input @ (ComposerInput::ModalInputChanged(_)
            | ComposerInput::ModalInputSubmitted(_)
            | ComposerInput::ModalInputCancelled
            | ComposerInput::ModalConfirmationAccepted(_)
            | ComposerInput::ModalConfirmationCancelled(_)),
        ) => {
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
            if let Some(effects) =
                super::super::modal_overlay::reduce_modal_input(input, &mut state)
            {
                queue_effects(work_sender, pending_effects, effects);
                *redraw = true;
            }
        }
        Ok(ComposerInput::Quit) | Err(TryRecvError::Disconnected) => {
            *should_quit = true;
        }
        Err(TryRecvError::Empty) => {}
    }
    Ok(false)
}
