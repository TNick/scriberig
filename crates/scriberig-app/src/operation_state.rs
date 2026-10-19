//! Stores operation records and publishes lifecycle events.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;

use crate::{ApplicationError, ApplicationEvent, OperationId, OperationRecord, OperationStatus};

/// Stores operation records and event subscribers behind one lock.
#[derive(Default)]
pub(super) struct OperationRegistryState {
    /// Operation records keyed by their stable identity.
    pub records: BTreeMap<OperationId, OperationRecord>,
    /// Cooperative cancellation flags keyed by operation identity.
    pub cancellation: BTreeMap<OperationId, Arc<AtomicBool>>,
    /// Subscribers receiving lifecycle events.
    subscribers: Vec<Sender<ApplicationEvent>>,
    /// Bounded in-process history available to protocol subscribers.
    pub history: Vec<(u64, ApplicationEvent)>,
    /// Next monotonic event cursor to allocate.
    next_event_sequence: u64,
}

/// Maximum number of lifecycle events retained for reconnect and inspection.
const EVENT_HISTORY_LIMIT: usize = 1024;

impl OperationRegistryState {
    /// Returns a mutable operation record or a structured not-found error.
    ///
    /// - `id`: Operation identity to resolve.
    ///
    /// A mutable operation record.
    ///
    /// Returns `NotFound` when the operation is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn record_mut(
        &mut self,
        id: &OperationId,
    ) -> Result<&mut OperationRecord, ApplicationError> {
        self.records
            .get_mut(id)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: id.to_string(),
            })
    }

    /// Publishes an event and removes disconnected subscribers.
    ///
    /// - `event`: Lifecycle event to publish.
    pub fn publish(&mut self, event: &ApplicationEvent) {
        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
        let sequence = self.next_event_sequence;
        self.history.push((sequence, event.clone()));
        if self.history.len() > EVENT_HISTORY_LIMIT {
            self.history.remove(0);
        }
        self.subscribers
            .retain(|subscriber| subscriber.send(event.clone()).is_ok());
    }

    /// Adds one lifecycle event subscriber.
    ///
    /// - `sender`: Channel receiving future lifecycle events.
    pub fn subscribe(&mut self, sender: Sender<ApplicationEvent>) {
        self.subscribers.push(sender);
    }
}

/// Converts a lifecycle transition to its transport-neutral event.
///
/// - `record`: Operation record describing the transition.
///
/// The corresponding application event.
/// # Returns
/// The operation result.
pub fn operation_event(record: &OperationRecord) -> ApplicationEvent {
    match record.status {
        OperationStatus::Completed => ApplicationEvent::OperationCompleted {
            operation: record.reference.clone(),
        },
        OperationStatus::Failed => ApplicationEvent::OperationFailed {
            operation: record.reference.clone(),
            error: record
                .error
                .as_ref()
                .map_or_else(|| "operation failed".to_owned(), ToString::to_string),
        },
        OperationStatus::Cancelled => ApplicationEvent::OperationCancelled {
            operation: record.reference.clone(),
        },
        OperationStatus::Queued | OperationStatus::Running => {
            if let Some(progress) = record.progress.clone() {
                ApplicationEvent::OperationProgressed {
                    operation: record.reference.clone(),
                    progress,
                }
            } else {
                ApplicationEvent::OperationStarted {
                    operation: record.reference.clone(),
                }
            }
        }
    }
}
