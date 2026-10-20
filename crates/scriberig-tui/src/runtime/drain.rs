//! Applies worker responses to TUI state and schedules follow-up work.

use std::{
    collections::VecDeque,
    io,
    sync::mpsc::{Receiver, SyncSender},
};

use crate::{action::Action, debug_log::TuiDebugLog, effect::Effect, state::TuiState};

use super::{RuntimeMessage, Work, queue_effects};

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

/// Applies completed worker messages and queues their follow-up work.
///
/// - `receiver`: Completed worker message queue.
/// - `state`: Mutable TUI state.
/// - `poll_in_flight`: Whether an event poll is active.
/// - `drain`: Mutable queues and diagnostics receiving message results.
///
/// Unit after all currently available messages have been applied.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an I/O error when a diagnostic write fails.
pub(super) fn drain_messages(
    receiver: &Receiver<RuntimeMessage>,
    state: &mut TuiState,
    poll_in_flight: &mut bool,
    drain: &mut MessageDrain<'_>,
) -> io::Result<()> {
    while let Ok(message) = receiver.try_recv() {
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
    Ok(())
}
