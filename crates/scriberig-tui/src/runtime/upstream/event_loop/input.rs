//! Dispatches upstream composer input to `ScribeRig` semantic actions.

use std::io;
use std::sync::mpsc;
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
#[path = "input/model_properties.rs"]
mod model_properties;
#[path = "input/selection.rs"]
mod selection;
#[path = "input/theme_and_edit.rs"]
mod theme_and_edit;

/// Borrows runtime state required to process one composer input event.
///
/// # Public members
/// None.
///
/// # Private members
/// - `service`: Application operations for model properties.
/// - `context`: Authorization context for application operations.
/// - `state`: Shared `ScribeRig` presentation state.
/// - `screen`: Upstream terminal screen.
/// - `editor`: Upstream composer editor.
/// - `composer`: `ScribeRig` composer adapter.
/// - `semantic_view`: `ScribeRig` semantic navigation adapter.
/// - `input_sender`: Channel for follow-up composer inputs.
/// - `work_sender`: Worker command channel.
/// - `pending_effects`: Effects waiting for worker capacity.
/// - `focus_before_overlay`: Focus target to restore after closing overlays.
/// - `shared_state`: Shared state used by upstream selectors.
/// - `debug_log`: Optional redacted input diagnostics.
/// - `should_quit`: Event loop termination flag.
/// - `redraw`: Whether the upstream screen needs a render request.
pub(super) struct InputContext<'a, 'log, S> {
    /// Application operations for model properties.
    pub(super) service: &'a S,
    /// Authorization context for application operations.
    pub(super) context: &'a scriberig_app::ApplicationContext,
    /// Shared `ScribeRig` presentation state.
    pub(super) state: &'a Arc<Mutex<TuiState>>,
    /// Upstream terminal screen.
    pub(super) screen: &'a Arc<TuiAltScreen>,
    /// Upstream composer editor.
    pub(super) editor: &'a Editor,
    /// `ScribeRig` composer adapter.
    pub(super) composer: &'a Arc<super::super::components::ComposerAdapter>,
    /// `ScribeRig` semantic navigation adapter.
    pub(super) semantic_view: &'a Arc<super::super::semantic_views::SemanticViewPanel>,
    /// Channel for follow-up composer inputs.
    pub(super) input_sender: &'a std::sync::mpsc::Sender<ComposerInput>,
    /// Worker command channel.
    pub(super) work_sender: &'a std::sync::mpsc::SyncSender<Work>,
    /// Model selector waiting for a model library response.
    pub(super) pending_selector: &'a mut Option<ControlKind>,
    /// Effects waiting for worker capacity.
    pub(super) pending_effects: &'a mut std::collections::VecDeque<Effect>,
    /// Focus target to restore after closing overlays.
    pub(super) focus_before_overlay: &'a mut Option<Arc<dyn Component>>,
    /// Shared state used by upstream selectors.
    pub(super) shared_state: &'a Arc<Mutex<TuiState>>,
    /// Optional redacted input diagnostics.
    pub(super) debug_log: &'a mut Option<&'log mut TuiDebugLog>,
    /// Event loop termination flag.
    pub(super) should_quit: &'a mut bool,
    /// Whether the upstream screen needs a render request.
    pub(super) redraw: &'a mut bool,
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
    input: Result<ComposerInput, std::sync::mpsc::RecvTimeoutError>,
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
            match prompt.trim() {
                "/model" => {
                    let _ = input_sender.send(ComposerInput::OpenControl(ControlKind::Model));
                    return Ok(true);
                }
                "/thinking" => {
                    let _ = input_sender.send(ComposerInput::OpenControl(ControlKind::Thinking));
                    return Ok(true);
                }
                "/theme" => {
                    let _ = input_sender.send(ComposerInput::OpenThemeSelector);
                    return Ok(true);
                }
                value if value.starts_with("/theme ") => {
                    if let Ok(mut state) = state.lock() {
                        state.conversation_composer.error =
                            Some("Use /theme without arguments.".to_owned());
                    }
                    screen.request_render(false);
                    return Ok(true);
                }
                _ => {}
            }
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
            state.conversation_composer.draft = prompt;
            let effects = crate::state::reduce(&mut state, Action::InputSubmitted);
            if let Some(log) = debug_log.as_deref_mut() {
                log.record_message("composer-submit", &state)?;
            }
            queue_effects(work_sender, pending_effects, effects);
            *should_quit = state.should_quit;
            *redraw = true;
        }
        Ok(ComposerInput::DraftChanged(draft)) => {
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
            model_properties::open(InputContext {
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
            });
        }
        Ok(
            input @ (ComposerInput::OpenThemeSelector
            | ComposerInput::BeginPropertyEdit { .. }
            | ComposerInput::OpenHelp
            | ComposerInput::CloseHelp
            | ComposerInput::CancelSelection),
        ) => {
            theme_and_edit::handle(
                InputContext {
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
                },
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
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
            crate::state::reduce(&mut state, Action::PaletteOpened);
            drop(state);
            let previous_focus = screen.get_focus();
            if super::super::selector_factory::open_palette_selector(
                screen,
                shared_state,
                input_sender,
            ) {
                if focus_before_overlay.is_none() {
                    *focus_before_overlay = previous_focus;
                }
                editor.set_focused(false);
            }
            *redraw = true;
        }
        Ok(ComposerInput::PaletteSelected(item_id)) => {
            restore_overlay_focus(screen, editor, focus_before_overlay);
            let routed_input = match item_id.as_str() {
                "model-picker" => Some(ComposerInput::OpenControl(ControlKind::Model)),
                "thinking-picker" => Some(ComposerInput::OpenControl(ControlKind::Thinking)),
                "theme-picker" => Some(ComposerInput::OpenThemeSelector),
                _ => None,
            };
            if let Some(input) = routed_input {
                if let Ok(mut state) = state.lock() {
                    state.overlay = None;
                }
                let _ = input_sender.send(input);
                return Ok(true);
            }
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
            let Some(crate::overlay::Overlay::Palette(palette)) = state.overlay.as_mut() else {
                return Ok(true);
            };
            if palette.select_id(&item_id) {
                let effects = crate::state::reduce(&mut state, Action::Activated);
                queue_effects(work_sender, pending_effects, effects);
            }
            *redraw = true;
        }
        Ok(ComposerInput::Selected(choice)) => {
            selection::handle(
                InputContext {
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
                },
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
        Ok(ComposerInput::Quit) | Err(mpsc::RecvTimeoutError::Disconnected) => {
            *should_quit = true;
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {}
    }
    Ok(false)
}
