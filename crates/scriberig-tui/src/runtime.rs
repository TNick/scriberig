//! Runs the event-driven terminal shell.

use std::io::{self, IsTerminal};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::time::Duration;

use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    TuiConfig,
    action::Action,
    effect::Effect,
    event,
    event::TuiEvent,
    render,
    service::{TuiService, execute_effect},
    state::{TuiState, reduce},
    terminal::TerminalGuard,
};
use scriberig_app::ApplicationContext;

/// Runs the interactive shell until a local quit event is received.
///
/// # Errors
/// Returns an I/O error when terminal setup, drawing, or event reading fails.
pub fn run<S: TuiService + Sync>(
    service: &S,
    context: &ApplicationContext,
    config: &TuiConfig,
) -> io::Result<()> {
    if !config.interactive {
        run_headless(service, context, config);
        return Ok(());
    }
    validate_interactive_terminal(
        config.interactive,
        io::stdin().is_terminal(),
        io::stdout().is_terminal(),
    )?;
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut state = TuiState::default();
    std::thread::scope(|scope| {
        let (work_sender, work_receiver) = mpsc::sync_channel(32);
        let (message_sender, message_receiver) = mpsc::sync_channel(32);
        let worker = scope.spawn(move || {
            worker_loop(service, context, &work_receiver, &message_sender);
        });
        let mut redraw = true;
        try_queue(&work_sender, Work::Effect(Effect::RequestCapabilities));
        if let Some(entity) = config.initial_entity {
            let request_id = state.next_request_id;
            state.next_request_id += 1;
            let effects = reduce(
                &mut state,
                crate::action::Action::InspectRequested {
                    request_id,
                    entity,
                    snapshot: config.snapshot,
                },
            );
            queue_effects(&work_sender, effects);
        }
        let mut event_cursor = 0_usize;
        let mut poll_in_flight = false;
        while !state.should_quit {
            drain_messages(
                &message_receiver,
                &mut state,
                &mut event_cursor,
                &mut poll_in_flight,
                &work_sender,
                &mut redraw,
            );
            if !poll_in_flight {
                poll_in_flight = try_queue(&work_sender, Work::PollApplicationEvents);
            }
            if redraw {
                terminal.draw(|frame| render::render(frame, &state))?;
                redraw = false;
            }
            let input = event::read_event(Duration::from_millis(250))?;
            if matches!(input, TuiEvent::Tick) {
                continue;
            }
            queue_effects(&work_sender, state.apply_event(&input));
            terminal.draw(|frame| render::render(frame, &state))?;
        }
        drop(work_sender);
        worker
            .join()
            .map_err(|_| io::Error::other("TUI worker panicked"))?;
        Ok(())
    })
}

/// Describes one unit of work executed outside the terminal thread.
enum Work {
    /// Execute one reducer-produced semantic effect.
    Effect(Effect),
    /// Poll the application event boundary.
    PollApplicationEvents,
}

/// Carries one completed worker operation back to the terminal thread.
enum RuntimeMessage {
    /// A reducer action produced by semantic work.
    Action(Box<Action>),
    /// Retained application events returned by a poll.
    ApplicationEvents(Vec<scriberig_app::ApplicationEvent>),
    /// A failed application event poll.
    ApplicationEventsFailed { category: String, message: String },
}

/// Runs semantic work until the terminal loop drops its work channel.
fn worker_loop<S: TuiService + Sync>(
    service: &S,
    context: &ApplicationContext,
    receiver: &Receiver<Work>,
    sender: &SyncSender<RuntimeMessage>,
) {
    while let Ok(work) = receiver.recv() {
        let message = match work {
            Work::Effect(effect) => {
                RuntimeMessage::Action(Box::new(execute_effect(service, context, effect)))
            }
            Work::PollApplicationEvents => match service.application_events(context) {
                Ok(events) => RuntimeMessage::ApplicationEvents(events),
                Err(error) => RuntimeMessage::ApplicationEventsFailed {
                    category: error.category().to_owned(),
                    message: crate::inspector::sanitize(&error.to_string()),
                },
            },
        };
        if sender.send(message).is_err() {
            break;
        }
    }
}

/// Queues one unit of work without allowing the UI thread to block.
///
/// # Parameters
/// - `sender`: Bounded worker queue.
/// - `work`: Work item to enqueue.
///
/// # Returns
/// Whether the work item was accepted.
fn try_queue(sender: &SyncSender<Work>, work: Work) -> bool {
    match sender.try_send(work) {
        Ok(()) => true,
        Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
    }
}

/// Queues all reducer effects that fit in the bounded worker queue.
///
/// # Parameters
/// - `sender`: Bounded worker queue.
/// - `effects`: Reducer effects to enqueue.
fn queue_effects(sender: &SyncSender<Work>, effects: Vec<Effect>) {
    for effect in effects {
        if !try_queue(sender, Work::Effect(effect)) {
            break;
        }
    }
}

/// Applies completed worker messages to the state and schedules follow-up work.
///
/// # Parameters
/// - `receiver`: Completed worker message queue.
/// - `state`: Mutable TUI state.
/// - `event_cursor`: Cursor into retained application events.
/// - `poll_in_flight`: Whether an event poll is active.
/// - `sender`: Bounded worker queue.
/// - `redraw`: Whether the terminal needs repainting.
fn drain_messages(
    receiver: &Receiver<RuntimeMessage>,
    state: &mut TuiState,
    event_cursor: &mut usize,
    poll_in_flight: &mut bool,
    sender: &SyncSender<Work>,
    redraw: &mut bool,
) {
    while let Ok(message) = receiver.try_recv() {
        match message {
            RuntimeMessage::Action(action) => {
                *redraw = true;
                queue_effects(sender, reduce(state, *action));
            }
            RuntimeMessage::ApplicationEvents(events) => {
                *poll_in_flight = false;
                if state.connection == crate::state::ConnectionState::Disconnected {
                    *redraw = true;
                    queue_effects(sender, reduce(state, Action::ConnectionRestored));
                }
                let start = (*event_cursor).min(events.len());
                let end = (start + 32).min(events.len());
                for application_event in &events[start..end] {
                    *redraw = true;
                    let event = TuiEvent::Application(application_event.clone());
                    queue_effects(sender, state.apply_event(&event));
                }
                *event_cursor = end;
            }
            RuntimeMessage::ApplicationEventsFailed { category, message } => {
                *poll_in_flight = false;
                *redraw = true;
                reduce(
                    state,
                    Action::OperationFailed {
                        request_id: 0,
                        category,
                        message,
                    },
                );
            }
        }
    }
}

/// Executes initial semantic work without taking ownership of a terminal.
///
/// # Parameters
/// - `service`: Application service used for semantic work.
/// - `context`: Caller capability and snapshot context.
/// - `config`: Initial-selection configuration.
///
/// # Returns
/// Success after the requested headless work has been dispatched.
///
/// # Errors
/// Returns an I/O error if the headless execution boundary fails.
fn run_headless<S: TuiService>(service: &S, context: &ApplicationContext, config: &TuiConfig) {
    let mut state = TuiState::default();
    let action = execute_effect(service, context, crate::effect::Effect::RequestCapabilities);
    reduce(&mut state, action);
    if let Some(entity) = config.initial_entity {
        let request_id = state.next_request_id;
        state.next_request_id += 1;
        let effects = reduce(
            &mut state,
            crate::action::Action::InspectRequested {
                request_id,
                entity,
                snapshot: config.snapshot,
            },
        );
        for effect in effects {
            let action = execute_effect(service, context, effect);
            reduce(&mut state, action);
        }
    }
}

/// Rejects interactive startup when the caller does not have a terminal.
/// - `interactive`: Argument named `interactive`.
/// - `stdin_is_terminal`: Argument named `stdin_is_terminal`.
/// - `stdout_is_terminal`: Argument named `stdout_is_terminal`.
///
/// # Returns
/// The value produced by the declaration.
///
/// # Errors
/// Returns an error when the operation cannot complete.
fn validate_interactive_terminal(
    interactive: bool,
    stdin_is_terminal: bool,
    stdout_is_terminal: bool,
) -> io::Result<()> {
    if interactive && (!stdin_is_terminal || !stdout_is_terminal) {
        return Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "interactive TUI requires a terminal on stdin and stdout",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use scriberig_app::{ApplicationContext, ApplicationError, InspectResult};

    use crate::TuiService;
    use crate::{
        event::{Key, KeyCode, Modifiers, TuiEvent},
        state::TuiState,
    };

    /// Provides an unsupported operation for runtime composition tests.
    struct Fixture;

    impl TuiService for Fixture {
        /// Rejects inspection while keeping the runtime test deterministic.
        ///
        /// # Parameters
        /// - `_context`: Test application context.
        /// - `_entity`: Test entity identity.
        /// - `_snapshot`: Optional test snapshot.
        ///
        /// # Returns
        /// Never returns a successful result.
        ///
        /// # Errors
        /// Always returns an unsupported-operation error.
        fn inspect_entity(
            &self,
            _context: &ApplicationContext,
            _entity: scriberig_core::EntityId,
            _snapshot: Option<scriberig_core::SnapshotId>,
        ) -> Result<InspectResult, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "test".to_owned(),
            })
        }
    }

    /// Verifies a runtime quit event is handled without a terminal backend.
    /// Verifies non-interactive composition avoids terminal takeover.
    /// Verifies headless execution does not take over a terminal.
    #[test]
    fn quit_event_terminates_state_loop() {
        let mut state = TuiState::default();
        state.apply_event(&TuiEvent::Key(Key {
            code: KeyCode::Char('q'),
            modifiers: Modifiers::default(),
        }));
        assert!(state.should_quit);
    }

    /// Verifies non-interactive startup is rejected before terminal takeover.
    #[test]
    fn rejects_noninteractive_terminal_for_interactive_runtime() {
        let error = super::validate_interactive_terminal(true, false, true)
            .expect_err("non-terminal input must be rejected");
        assert_eq!(error.kind(), std::io::ErrorKind::NotConnected);
    }

    /// Verifies headless composition does not require an attached terminal.
    #[test]
    fn accepts_noninteractive_composition_without_terminal() {
        super::validate_interactive_terminal(false, false, false)
            .expect("headless composition must remain usable");
    }

    /// Verifies headless startup does not enter terminal mode.
    #[test]
    fn noninteractive_run_does_not_take_over_terminal() {
        let result = super::run(
            &Fixture,
            &ApplicationContext::empty(),
            &crate::TuiConfig::default(),
        );
        assert!(result.is_ok());
    }
}
