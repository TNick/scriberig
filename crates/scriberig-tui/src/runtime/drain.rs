//! Applies worker responses to TUI state and schedules follow-up work.

use std::{
    collections::VecDeque,
    io,
    sync::mpsc::{Receiver, SyncSender},
};

use crate::{action::Action, debug_log::TuiDebugLog, effect::Effect, state::TuiState};

use super::{RuntimeMessage, Work, queue_effects};

/// Maximum worker messages processed before the upstream TUI gets another tick.
pub(super) const MAX_MESSAGES_PER_TICK: usize = 16;

/// Holds the queues and redraw state updated while receiving worker messages.
pub(super) struct MessageDrain<'a> {
    /// Bounded worker queue receiving follow-up effects.
    pub(super) sender: &'a SyncSender<Work>,
    /// Effects waiting for worker queue capacity.
    pub(super) pending_effects: &'a mut VecDeque<Effect>,
    /// Application events waiting for reducer processing.
    pub(super) pending_events: &'a mut VecDeque<scriberig_app::ApplicationEvent>,
    /// Whether the terminal needs repainting.
    pub(super) redraw: &'a mut bool,
    /// Optional content-redacted diagnostic destination.
    pub(super) debug_log: Option<&'a mut TuiDebugLog>,
}

/// Applies a bounded worker-message batch and queues its follow-up work.
///
/// - `receiver`: Completed worker message queue.
/// - `state`: Mutable TUI state.
/// - `poll_in_flight`: Whether an event poll is active.
/// - `drain`: Mutable queues and diagnostics receiving message results.
///
/// Number of messages applied before the channel was empty or the tick limit was reached.
/// # Returns
/// The number of messages applied in this tick.
/// # Errors
/// Returns an I/O error when a diagnostic write fails.
pub(super) fn drain_messages(
    receiver: &Receiver<RuntimeMessage>,
    state: &mut TuiState,
    poll_in_flight: &mut bool,
    drain: &mut MessageDrain<'_>,
) -> io::Result<usize> {
    let mut processed = 0;
    while processed < MAX_MESSAGES_PER_TICK {
        let Ok(message) = receiver.try_recv() else {
            break;
        };
        processed += 1;
        match message {
            RuntimeMessage::Action(action) => {
                *drain.redraw = true;
                queue_effects(
                    drain.sender,
                    drain.pending_effects,
                    crate::state::reduce(state, *action),
                );
                if let Some(log) = drain.debug_log.as_deref_mut() {
                    log.record_message("action-result", state)?;
                }
            }
            RuntimeMessage::ApplicationEvents(events) => {
                *poll_in_flight = false;
                if state.connection == crate::state::ConnectionState::Disconnected {
                    *drain.redraw = true;
                    queue_effects(
                        drain.sender,
                        drain.pending_effects,
                        crate::state::reduce(state, Action::ConnectionRestored),
                    );
                }
                let event_count = events.len();
                drain.pending_events.extend(events);
                if event_count > 0
                    && let Some(log) = drain.debug_log.as_deref_mut()
                {
                    log.record_message(&format!("application-events:{event_count}"), state)?;
                }
            }
            RuntimeMessage::ApplicationEventsFailed { category, message } => {
                *poll_in_flight = false;
                *drain.redraw = true;
                crate::state::reduce(
                    state,
                    Action::OperationFailed {
                        request_id: 0,
                        category: category.clone(),
                        message,
                    },
                );
                if let Some(log) = drain.debug_log.as_deref_mut() {
                    log.record_message(&format!("application-events-failed:{category}"), state)?;
                }
            }
        }
    }
    Ok(processed)
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::sync::mpsc;

    use super::*;
    use crate::{action::Action, state::TuiState};

    /// Builds expected text for a sequence of transient worker messages.
    ///
    /// # Parameters
    /// - `count`: Number of ordered message fragments.
    ///
    /// # Returns
    /// The concatenated worker message text.
    fn message_text(count: usize) -> String {
        let mut text = String::new();
        for index in 0..count {
            write!(text, "message-{index}|").expect("write into string");
        }
        text
    }

    /// Verifies a busy worker cannot monopolize one terminal tick.
    #[test]
    fn worker_messages_are_bounded_per_tick() {
        let (message_sender, receiver) = mpsc::channel();
        let operation = scriberig_core::OperationId::new("streaming").expect("operation id");
        for index in 0..(MAX_MESSAGES_PER_TICK + 3) {
            message_sender
                .send(RuntimeMessage::Action(Box::new(
                    Action::AgentOutputObserved {
                        operation: operation.clone(),
                        text: format!("message-{index}|"),
                        complete: false,
                    },
                )))
                .expect("queue worker message");
        }
        let (work_sender, _work_receiver) = mpsc::sync_channel(0);
        let mut pending_effects = VecDeque::new();
        let mut pending_events = VecDeque::new();
        let mut redraw = false;
        let mut state = TuiState {
            conversation_browsing: true,
            ..TuiState::default()
        };
        let mut poll_in_flight = false;
        let mut drain = MessageDrain {
            sender: &work_sender,
            pending_effects: &mut pending_effects,
            pending_events: &mut pending_events,
            redraw: &mut redraw,
            debug_log: None,
        };

        let first_batch = drain_messages(&receiver, &mut state, &mut poll_in_flight, &mut drain)
            .expect("drain first worker batch");

        assert_eq!(first_batch, MAX_MESSAGES_PER_TICK);
        assert_eq!(
            state.transient_agent_outputs[0].text,
            message_text(MAX_MESSAGES_PER_TICK)
        );
        assert!(
            state.conversation_browsing,
            "streaming must preserve browsing mode"
        );
        assert!(
            *drain.redraw,
            "a bounded batch must schedule a visible refresh"
        );

        let second_batch = drain_messages(&receiver, &mut state, &mut poll_in_flight, &mut drain)
            .expect("drain remaining worker messages");
        assert_eq!(second_batch, 3);
        assert_eq!(
            state.transient_agent_outputs[0].text,
            message_text(MAX_MESSAGES_PER_TICK + 3)
        );
    }
}
