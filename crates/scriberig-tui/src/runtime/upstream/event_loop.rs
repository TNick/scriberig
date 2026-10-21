//! Applies `ScribeRig` reducers and effects during the upstream TUI lifecycle.

use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use rpi_tui::{Component, Editor, Focusable, TUI, TuiAltScreen};

use crate::action::Action;
use crate::debug_log::TuiDebugLog;
use crate::effect::Effect;
use crate::service::TuiService;
use crate::state::TuiState;

#[path = "event_loop/focus.rs"]
pub(super) mod focus;
#[path = "event_loop/input.rs"]
mod input;

use super::super::{
    MessageDrain, Work, dispatch_pending_events, flush_effects, queue_effects, try_queue,
    worker_loop,
};
use super::ComposerInput;
use super::modal_overlay::ModalOverlayController;
use super::selector_factory::open_control_selector;

/// Borrows all inputs needed by the upstream-owned `ScribeRig` event loop.
///
/// # Public members
/// None.
///
/// # Private members
/// - `service`: `ScribeRig` application operations.
/// - `context`: Authorization and snapshot context.
/// - `config`: Initial selection and project scope.
/// - `state`: Shared presentation state.
/// - `screen`: Upstream terminal runtime.
/// - `editor`: Upstream composer editor.
/// - `composer`: Upstream-focusable `ScribeRig` composer adapter.
/// - `semantic_view`: Upstream-focusable semantic navigation component.
/// - `input_sender`: Composer-to-runtime input channel.
/// - `input_receiver`: Runtime input channel receiver.
/// - `debug_log`: Optional redacted diagnostics.
pub(super) struct EventLoopContext<'a, S> {
    /// `ScribeRig` application operations.
    pub(super) service: &'a S,
    /// Authorization and snapshot context.
    pub(super) context: &'a scriberig_app::ApplicationContext,
    /// Initial selection and project scope.
    pub(super) config: &'a crate::TuiConfig,
    /// Shared presentation state.
    pub(super) state: &'a Arc<Mutex<TuiState>>,
    /// Upstream terminal runtime.
    pub(super) screen: &'a Arc<TuiAltScreen>,
    /// Upstream composer editor.
    pub(super) editor: &'a Editor,
    /// Upstream-focusable `ScribeRig` composer adapter.
    pub(super) composer: &'a Arc<super::components::ComposerAdapter>,
    /// Upstream-focusable semantic navigation component.
    pub(super) semantic_view: &'a Arc<super::semantic_views::SemanticViewPanel>,
    /// Composer-to-runtime input channel.
    pub(super) input_sender: &'a Sender<ComposerInput>,
    /// Runtime input channel receiver.
    pub(super) input_receiver: &'a Receiver<ComposerInput>,
    /// Optional redacted diagnostics.
    pub(super) debug_log: Option<&'a mut TuiDebugLog>,
}

/// Applies application work while upstream owns terminal input and rendering.
///
/// # Parameters
/// - `event_loop`: Borrowed services and input channels for the loop.
///
/// # Returns
/// Success after the quit event and worker shutdown.
///
/// # Errors
/// Returns an error when application diagnostics or worker shutdown fails.
#[allow(clippy::too_many_lines)] // Keeps ordered event/effect handling together.
pub(super) fn run<S: TuiService + Sync>(event_loop: EventLoopContext<'_, S>) -> io::Result<()> {
    let EventLoopContext {
        service,
        context,
        config,
        state,
        screen,
        editor,
        composer,
        semantic_view,
        input_sender,
        input_receiver,
        mut debug_log,
    } = event_loop;
    let shared_state = Arc::clone(state);
    thread::scope(|scope| {
        let (work_sender, work_receiver) = mpsc::sync_channel(32);
        let (message_sender, message_receiver) = mpsc::sync_channel(32);
        let worker_scope = scope;
        let worker_context = context.clone();
        let worker = scope.spawn(move || {
            worker_loop(
                worker_scope,
                service,
                worker_context,
                work_receiver,
                message_sender,
            );
        });
        let mut pending_effects = std::collections::VecDeque::new();
        let mut pending_events = std::collections::VecDeque::new();
        let mut pending_selector = None;
        let mut modal_overlay = ModalOverlayController::new();
        let mut focus_before_overlay: Option<Arc<dyn Component>> = None;
        let mut poll_in_flight = false;
        {
            let mut state = state
                .lock()
                .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
            queue_effects(
                &work_sender,
                &mut pending_effects,
                vec![
                    Effect::RequestCapabilities,
                    Effect::ReadPresentationPreferences,
                ],
            );
            if let Some(entity) = config.initial_entity {
                let request_id = state.next_request_id;
                state.next_request_id = state.next_request_id.saturating_add(1);
                let effects = crate::state::reduce(
                    &mut state,
                    Action::InspectRequested {
                        request_id,
                        entity,
                        snapshot: config.snapshot,
                    },
                );
                queue_effects(&work_sender, &mut pending_effects, effects);
            }
        }

        let mut should_quit = false;
        while !should_quit {
            let mut redraw = false;
            let mut selector_to_open = None;
            {
                let mut state = state
                    .lock()
                    .map_err(|_| io::Error::other("TUI state lock poisoned"))?;
                {
                    let mut drain = MessageDrain {
                        sender: &work_sender,
                        pending_effects: &mut pending_effects,
                        pending_events: &mut pending_events,
                        redraw: &mut redraw,
                        debug_log: debug_log.as_deref_mut(),
                    };
                    super::super::drain::drain_messages(
                        &message_receiver,
                        &mut state,
                        &mut poll_in_flight,
                        &mut drain,
                    )?;
                }
                dispatch_pending_events(
                    &mut state,
                    &mut pending_events,
                    &work_sender,
                    &mut pending_effects,
                    &mut redraw,
                );
                flush_effects(&work_sender, &mut pending_effects);
                if !poll_in_flight && pending_events.is_empty() {
                    poll_in_flight = try_queue(&work_sender, Work::PollApplicationEvents);
                }
                if !state.conversation_composer.draft.is_empty()
                    && editor.get_expanded_text().is_empty()
                {
                    editor.set_text(&state.conversation_composer.draft);
                    redraw = true;
                }
                if let Some(kind) = pending_selector {
                    match &state.model_library {
                        crate::inspector::LoadState::Ready(library) => {
                            let library = library.clone();
                            state.overlay = None;
                            pending_selector = None;
                            selector_to_open = Some((kind, library));
                        }
                        crate::inspector::LoadState::Failed { .. } => {
                            pending_selector = None;
                        }
                        _ => {}
                    }
                }
                should_quit = state.should_quit;
            }
            if let Some((kind, library)) = selector_to_open {
                let previous_focus = screen.get_focus();
                if open_control_selector(screen, state, kind, &library, None, input_sender) {
                    if focus_before_overlay.is_none() {
                        focus_before_overlay = previous_focus;
                    }
                    editor.set_focused(false);
                }
                redraw = true;
            }
            modal_overlay.synchronize(
                screen,
                state,
                editor,
                input_sender,
                &mut focus_before_overlay,
            );

            if input::handle(
                input::InputContext {
                    service,
                    context,
                    state,
                    screen,
                    editor,
                    composer,
                    semantic_view,
                    input_sender,
                    work_sender: &work_sender,
                    pending_selector: &mut pending_selector,
                    pending_effects: &mut pending_effects,
                    focus_before_overlay: &mut focus_before_overlay,
                    shared_state: &shared_state,
                    debug_log: &mut debug_log,
                    should_quit: &mut should_quit,
                    redraw: &mut redraw,
                },
                input_receiver.recv_timeout(Duration::from_millis(100)),
            )? {
                continue;
            }
            if redraw {
                screen.request_render(false);
            }
        }

        drop(work_sender);
        drop(message_receiver);
        worker
            .join()
            .map_err(|_| io::Error::other("TUI worker panicked"))?;
        Ok(())
    })
}
