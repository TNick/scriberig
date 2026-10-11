//! Defines stable long-running application operation state.

pub use scriberig_core::OperationId;

use crate::{ApplicationError, ApplicationEvent};
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// Classifies the semantic work performed by an operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKind {
    /// Inspects one semantic target.
    Inspect,
    /// Lists a typed collection.
    List,
    /// Follows a typed relation.
    Follow,
    /// Discovers semantic entities.
    Find,
    /// Evaluates a structural query.
    Query,
    /// Computes a semantic diff.
    Diff,
    /// Reads change history.
    History,
    /// Renders a semantic target through a registered view.
    Render,
    /// Creates or updates a branch context.
    Branch,
    /// Applies a proposed change.
    Apply,
    /// Commits a validated change.
    Commit,
}

/// Describes the lifecycle state of an operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationStatus {
    /// Operation is waiting to start.
    Queued,
    /// Operation is currently running.
    Running,
    /// Operation completed successfully.
    Completed,
    /// Operation failed with a structured application error.
    Failed,
    /// Operation was cancelled before completion.
    Cancelled,
}

impl OperationStatus {
    /// Reports whether the operation will not produce further progress.
    ///
    /// # Parameters
    /// - `self`: Status to classify.
    ///
    /// # Returns
    /// `true` for completed, failed, and cancelled states.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Carries concise progress information without embedding mutable result data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProgress {
    /// Current operation status.
    pub status: OperationStatus,
    /// Optional completed work count.
    pub completed: Option<u64>,
    /// Optional total work count.
    pub total: Option<u64>,
    /// Optional human-readable phase name.
    pub phase: Option<String>,
}

/// Retains the inspectable state of one tracked application operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationRecord {
    /// Stable operation identity.
    pub reference: OperationId,
    /// Universal operation family.
    pub kind: OperationKind,
    /// Exact base snapshot, when the operation has one.
    pub base_snapshot: Option<scriberig_core::SnapshotId>,
    /// Optional named branch context.
    pub branch: Option<scriberig_core::RootName>,
    /// Current lifecycle state.
    pub status: OperationStatus,
    /// Optional coarse progress information.
    pub progress: Option<OperationProgress>,
    /// Stable handles produced by a completed operation.
    pub result_handles: Vec<String>,
    /// Structured failure when the operation failed.
    pub error: Option<crate::ApplicationError>,
}

/// Owns in-process operation state and publishes lifecycle observations.
#[derive(Clone, Default)]
pub struct OperationRegistry {
    /// Shared mutable lifecycle state and event subscribers.
    state: Arc<Mutex<OperationRegistryState>>,
}

/// Stores operation records and event subscribers behind one lock.
#[derive(Default)]
struct OperationRegistryState {
    /// Operation records keyed by their stable identity.
    records: BTreeMap<OperationId, OperationRecord>,
    /// Subscribers receiving lifecycle events.
    subscribers: Vec<Sender<ApplicationEvent>>,
    /// Bounded in-process history available to protocol subscribers.
    history: Vec<ApplicationEvent>,
}

impl OperationRegistry {
    /// Creates an empty operation registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a queued operation.
    ///
    /// # Parameters
    /// - `id`: Stable operation identity.
    /// - `kind`: Semantic operation family.
    ///
    /// # Returns
    /// The inserted queued operation.
    ///
    /// # Errors
    /// Returns an error when the identity is already registered.
    pub fn start(
        &self,
        id: OperationId,
        kind: OperationKind,
    ) -> Result<OperationRecord, ApplicationError> {
        let mut state = self.lock()?;
        if state.records.contains_key(&id) {
            return Err(ApplicationError::OperationFailed {
                operation: id.to_string(),
                message: "operation identity is already registered".to_owned(),
            });
        }
        let record = OperationRecord::queued(id.clone(), kind);
        state.records.insert(id.clone(), record.clone());
        state.publish(&ApplicationEvent::OperationStarted { operation: id });
        Ok(record)
    }

    /// Returns the current record for one operation.
    ///
    /// # Parameters
    /// - `id`: Operation identity to resolve.
    ///
    /// # Returns
    /// A snapshot of the current operation record.
    ///
    /// # Errors
    /// Returns `NotFound` when the operation is unknown.
    pub fn get(&self, id: &OperationId) -> Result<OperationRecord, ApplicationError> {
        let state = self.lock()?;
        state
            .records
            .get(id)
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                subject: id.to_string(),
            })
    }

    /// Transitions an operation to a new lifecycle state.
    ///
    /// # Parameters
    /// - `id`: Operation identity to update.
    /// - `status`: New lifecycle status.
    ///
    /// # Returns
    /// The updated operation record.
    ///
    /// # Errors
    /// Returns a structured failure for unknown or terminal operations.
    pub fn transition(
        &self,
        id: &OperationId,
        status: OperationStatus,
    ) -> Result<OperationRecord, ApplicationError> {
        let mut state = self.lock()?;
        let record = state.record_mut(id)?;
        let updated = record.clone().transition(status)?;
        *record = updated.clone();
        state.publish(&operation_event(&updated));
        Ok(updated)
    }

    /// Records bounded progress for an active operation.
    ///
    /// # Parameters
    /// - `id`: Operation identity to update.
    /// - `progress`: New progress observation.
    ///
    /// # Returns
    /// The updated operation record.
    ///
    /// # Errors
    /// Returns a structured failure for unknown or terminal operations.
    pub fn progress(
        &self,
        id: &OperationId,
        progress: OperationProgress,
    ) -> Result<OperationRecord, ApplicationError> {
        let mut state = self.lock()?;
        let record = state.record_mut(id)?;
        if record.status.is_terminal() {
            return Err(ApplicationError::OperationFailed {
                operation: id.to_string(),
                message: "terminal operation cannot report progress".to_owned(),
            });
        }
        record.progress = Some(progress.clone());
        let updated = record.clone();
        state.publish(&ApplicationEvent::OperationProgressed {
            operation: id.clone(),
            progress,
        });
        Ok(updated)
    }

    /// Marks an operation completed with stable result handles.
    ///
    /// # Parameters
    /// - `id`: Operation identity to complete.
    /// - `result_handles`: Stable references produced by the operation.
    ///
    /// # Returns
    /// The completed operation record.
    ///
    /// # Errors
    /// Returns a structured failure when completion is invalid.
    pub fn complete(
        &self,
        id: &OperationId,
        result_handles: Vec<String>,
    ) -> Result<OperationRecord, ApplicationError> {
        self.transition(id, OperationStatus::Completed)?;
        let mut state = self.lock()?;
        let record = state.record_mut(id)?;
        record.result_handles = result_handles;
        let updated = record.clone();
        state.publish(&ApplicationEvent::OperationCompleted {
            operation: id.clone(),
        });
        Ok(updated)
    }

    /// Cancels one active operation.
    ///
    /// # Parameters
    /// - `id`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled operation record.
    ///
    /// # Errors
    /// Returns a structured failure when cancellation is invalid.
    pub fn cancel(&self, id: &OperationId) -> Result<OperationRecord, ApplicationError> {
        self.transition(id, OperationStatus::Cancelled)
    }

    /// Subscribes to future lifecycle events.
    #[must_use]
    pub fn subscribe(&self) -> Receiver<ApplicationEvent> {
        let (sender, receiver) = mpsc::channel();
        if let Ok(mut state) = self.state.lock() {
            state.subscribers.push(sender);
        }
        receiver
    }

    /// Returns lifecycle events published so far.
    #[must_use]
    pub fn events(&self) -> Vec<ApplicationEvent> {
        self.state
            .lock()
            .map(|state| state.history.clone())
            .unwrap_or_default()
    }

    /// Acquires the registry state or reports a lock failure.
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, OperationRegistryState>, ApplicationError> {
        self.state.lock().map_err(|_| ApplicationError::Internal {
            message: "operation registry lock failed".to_owned(),
        })
    }
}

impl OperationRegistryState {
    /// Returns a mutable operation record or a structured not-found error.
    fn record_mut(&mut self, id: &OperationId) -> Result<&mut OperationRecord, ApplicationError> {
        self.records
            .get_mut(id)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: id.to_string(),
            })
    }

    /// Publishes an event and removes disconnected subscribers.
    fn publish(&mut self, event: &ApplicationEvent) {
        self.history.push(event.clone());
        self.subscribers
            .retain(|subscriber| subscriber.send(event.clone()).is_ok());
    }
}

/// Converts a lifecycle transition to its transport-neutral event.
fn operation_event(record: &OperationRecord) -> ApplicationEvent {
    if record.status.is_terminal() {
        ApplicationEvent::OperationCompleted {
            operation: record.reference.clone(),
        }
    } else if let Some(progress) = record.progress.clone() {
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

impl OperationRecord {
    /// Creates a queued operation record.
    ///
    /// # Parameters
    /// - `reference`: Stable operation identity.
    /// - `kind`: Universal operation family.
    ///
    /// # Returns
    /// A queued operation record without mutable result data.
    #[must_use]
    pub fn queued(reference: OperationId, kind: OperationKind) -> Self {
        Self {
            reference,
            kind,
            base_snapshot: None,
            branch: None,
            status: OperationStatus::Queued,
            progress: None,
            result_handles: Vec::new(),
            error: None,
        }
    }

    /// Transitions the operation to a new lifecycle state.
    ///
    /// # Parameters
    /// - `self`: Operation record to update.
    /// - `status`: Next lifecycle status.
    ///
    /// # Returns
    /// The updated operation record.
    ///
    /// # Errors
    /// Returns an application error when a terminal operation is changed.
    pub fn transition(mut self, status: OperationStatus) -> Result<Self, crate::ApplicationError> {
        if self.status.is_terminal() {
            return Err(crate::ApplicationError::OperationFailed {
                operation: self.reference.to_string(),
                message: "terminal operation cannot transition".to_owned(),
            });
        }
        self.status = status;
        if status.is_terminal() {
            self.progress = Some(OperationProgress {
                status,
                completed: self.progress.as_ref().and_then(|value| value.completed),
                total: self.progress.as_ref().and_then(|value| value.total),
                phase: self.progress.as_ref().and_then(|value| value.phase.clone()),
            });
        }
        Ok(self)
    }
}
