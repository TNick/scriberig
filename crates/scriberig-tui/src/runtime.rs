//! Runs the event-driven terminal shell.

use std::collections::VecDeque;
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

/// Maximum lifecycle events reduced before returning to terminal input handling.
const APPLICATION_EVENT_BATCH_SIZE: usize = 128;

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
    let mut state = TuiState {
        project: config.project.clone(),
        ..TuiState::default()
    };
    std::thread::scope(|scope| {
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
        let mut redraw = true;
        let mut pending_effects = VecDeque::new();
        queue_effects(
            &work_sender,
            &mut pending_effects,
            vec![Effect::RequestCapabilities],
        );
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
            queue_effects(&work_sender, &mut pending_effects, effects);
        }
        let mut pending_events = VecDeque::new();
        let mut poll_in_flight = false;
        while !state.should_quit {
            drain_messages(
                &message_receiver,
                &mut state,
                &mut poll_in_flight,
                &work_sender,
                &mut pending_effects,
                &mut pending_events,
                &mut redraw,
            );
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
            if redraw {
                terminal.draw(|frame| render::render(frame, &state))?;
                redraw = false;
            }
            let input = event::read_event(Duration::from_millis(250))?;
            if matches!(input, TuiEvent::Tick) {
                continue;
            }
            queue_effects(
                &work_sender,
                &mut pending_effects,
                state.apply_event(&input),
            );
            terminal.draw(|frame| render::render(frame, &state))?;
        }
        drop(work_sender);
        drop(message_receiver);
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
#[allow(clippy::needless_pass_by_value)] // The scoped worker owns these values for its full run.
fn worker_loop<'scope, S: TuiService + Sync>(
    scope: &'scope std::thread::Scope<'scope, '_>,
    service: &'scope S,
    context: ApplicationContext,
    receiver: Receiver<Work>,
    sender: SyncSender<RuntimeMessage>,
) {
    while let Ok(work) = receiver.recv() {
        if let Work::Effect(effect @ Effect::SubmitAgent { .. }) = work {
            let sender = sender.clone();
            let context = context.clone();
            scope.spawn(move || {
                let action = execute_effect(service, &context, effect);
                let _ = sender.send(RuntimeMessage::Action(Box::new(action)));
            });
            continue;
        }
        let message = match work {
            Work::Effect(effect) => {
                RuntimeMessage::Action(Box::new(execute_effect(service, &context, effect)))
            }
            Work::PollApplicationEvents => match service.application_events(&context) {
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

/// Retains reducer effects and queues as many as the bounded worker accepts.
///
/// # Parameters
/// - `sender`: Bounded worker queue.
/// - `pending`: Effects retained until worker capacity returns.
/// - `effects`: Reducer effects to enqueue.
fn queue_effects(sender: &SyncSender<Work>, pending: &mut VecDeque<Effect>, effects: Vec<Effect>) {
    pending.extend(effects);
    flush_effects(sender, pending);
}

/// Queues retained effects until the bounded worker queue is full.
///
/// # Parameters
/// - `sender`: Bounded worker queue.
/// - `pending`: Effects retained until worker capacity returns.
fn flush_effects(sender: &SyncSender<Work>, pending: &mut VecDeque<Effect>) {
    while let Some(effect) = pending.pop_front() {
        match sender.try_send(Work::Effect(effect)) {
            Ok(()) => {}
            Err(TrySendError::Full(Work::Effect(effect))) => {
                pending.push_front(effect);
                break;
            }
            Err(TrySendError::Disconnected(_)) => break,
            Err(TrySendError::Full(Work::PollApplicationEvents)) => {
                unreachable!("only effects are queued through this helper")
            }
        }
    }
}

/// Applies completed worker messages to the state and schedules follow-up work.
///
/// # Parameters
/// - `receiver`: Completed worker message queue.
/// - `state`: Mutable TUI state.
/// - `poll_in_flight`: Whether an event poll is active.
/// - `sender`: Bounded worker queue.
/// - `pending_effects`: Effects retained until worker capacity returns.
/// - `pending_events`: Lifecycle events retained until dispatched.
/// - `redraw`: Whether the terminal needs repainting.
fn drain_messages(
    receiver: &Receiver<RuntimeMessage>,
    state: &mut TuiState,
    poll_in_flight: &mut bool,
    sender: &SyncSender<Work>,
    pending_effects: &mut VecDeque<Effect>,
    pending_events: &mut VecDeque<scriberig_app::ApplicationEvent>,
    redraw: &mut bool,
) {
    while let Ok(message) = receiver.try_recv() {
        match message {
            RuntimeMessage::Action(action) => {
                *redraw = true;
                queue_effects(sender, pending_effects, reduce(state, *action));
            }
            RuntimeMessage::ApplicationEvents(events) => {
                *poll_in_flight = false;
                if state.connection == crate::state::ConnectionState::Disconnected {
                    *redraw = true;
                    queue_effects(
                        sender,
                        pending_effects,
                        reduce(state, Action::ConnectionRestored),
                    );
                }
                pending_events.extend(events);
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

/// Applies one bounded batch of retained application lifecycle events.
///
/// # Parameters
/// - `state`: TUI state receiving event reductions.
/// - `pending_events`: Events waiting for reducer processing.
/// - `sender`: Bounded worker queue for resulting semantic effects.
/// - `pending_effects`: Effects retained until worker capacity returns.
/// - `redraw`: Whether visible state changed.
fn dispatch_pending_events(
    state: &mut TuiState,
    pending_events: &mut VecDeque<scriberig_app::ApplicationEvent>,
    sender: &SyncSender<Work>,
    pending_effects: &mut VecDeque<Effect>,
    redraw: &mut bool,
) {
    for _ in 0..APPLICATION_EVENT_BATCH_SIZE {
        let Some(application_event) = pending_events.pop_front() else {
            break;
        };
        *redraw = true;
        let event = TuiEvent::Application(application_event);
        queue_effects(sender, pending_effects, state.apply_event(&event));
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
#[path = "runtime_tests.rs"]
mod tests;
