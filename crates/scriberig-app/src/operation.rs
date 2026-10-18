//! Defines stable long-running application operation state.

pub use scriberig_core::OperationId;

use crate::{ApplicationError, ApplicationEvent};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

#[path = "operation_agent.rs"]
mod operation_agent;
#[path = "operation_cancellation.rs"]
mod operation_cancellation;
#[path = "operation_record.rs"]
mod operation_record;
#[path = "operation_state.rs"]
mod operation_state;
#[path = "operation_session_event.rs"]
mod session_event;
use operation_state::{OperationRegistryState, operation_event};

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
    /// Executes a model-driven agent operation.
    Agent,
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
    /// Structured terminal agent result, when this is an agent operation.
    pub agent_result: Option<crate::AgentResult>,
    /// Structured failure when the operation failed.
    pub error: Option<crate::ApplicationError>,
}

/// Owns in-process operation state and publishes lifecycle observations.
#[derive(Clone, Default)]
pub struct OperationRegistry {
    /// Shared mutable lifecycle state and event subscribers.
    pub(in crate::operation) state: Arc<Mutex<OperationRegistryState>>,
}

impl OperationRegistry {
    /// Creates an empty operation registry.
    ///
    /// # Returns
    /// A new empty operation registry.
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
        state
            .cancellation
            .insert(id.clone(), Arc::new(AtomicBool::new(false)));
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

    /// Lists the current in-process operation records in stable identity order.
    ///
    /// # Returns
    /// A snapshot of all tracked operations.
    #[must_use]
    pub fn list(&self) -> Vec<OperationRecord> {
        self.state
            .lock()
            .map(|state| state.records.values().cloned().collect())
            .unwrap_or_default()
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

    /// Marks an operation failed and retains its structured error.
    ///
    /// # Parameters
    /// - `id`: Operation identity to update.
    /// - `error`: Structured failure explaining the terminal state.
    ///
    /// # Returns
    /// The failed operation record.
    ///
    /// # Errors
    /// Returns a structured failure when the operation is unknown or terminal.
    pub fn fail(
        &self,
        id: &OperationId,
        error: ApplicationError,
    ) -> Result<OperationRecord, ApplicationError> {
        let mut state = self.lock()?;
        let record = state.record_mut(id)?;
        let mut updated = record.clone().transition(OperationStatus::Failed)?;
        updated.error = Some(error);
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

    /// Completes an agent operation with its structured durable result.
    ///
    /// # Parameters
    /// - `id`: Operation identity to complete.
    /// - `result_handles`: Stable semantic handles produced by the operation.
    /// - `result`: Provider-neutral terminal agent result.
    ///
    /// # Returns
    /// The completed operation record.
    ///
    /// # Errors
    /// Returns a structured failure when completion is invalid.
    pub fn complete_agent(
        &self,
        id: &OperationId,
        result_handles: Vec<String>,
        result: crate::AgentResult,
    ) -> Result<OperationRecord, ApplicationError> {
        self.transition(id, OperationStatus::Completed)?;
        let mut state = self.lock()?;
        let record = state.record_mut(id)?;
        record.result_handles = result_handles;
        record.agent_result = Some(result);
        let updated = record.clone();
        state.publish(&ApplicationEvent::OperationCompleted {
            operation: id.clone(),
        });
        Ok(updated)
    }

    /// Subscribes to future lifecycle events.
    ///
    /// # Returns
    /// A receiver for future lifecycle events.
    #[must_use]
    pub fn subscribe(&self) -> Receiver<ApplicationEvent> {
        let (sender, receiver) = mpsc::channel();
        if let Ok(mut state) = self.state.lock() {
            state.subscribe(sender);
        }
        receiver
    }

    /// Returns lifecycle events published so far.
    ///
    /// # Returns
    /// A snapshot of all retained lifecycle events.
    #[must_use]
    pub fn events(&self) -> Vec<ApplicationEvent> {
        self.state
            .lock()
            .map(|state| {
                state
                    .history
                    .iter()
                    .map(|(_, event)| event.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns retained lifecycle events with their stable cursors.
    ///
    /// # Returns
    /// Events paired with the monotonic cursor allocated at publication time.
    #[must_use]
    pub fn events_with_sequences(&self) -> Vec<(u64, ApplicationEvent)> {
        self.state
            .lock()
            .map(|state| state.history.clone())
            .unwrap_or_default()
    }

    /// Acquires the registry state or reports a lock failure.
    ///
    /// # Parameters
    /// - `self`: Registry whose state is locked.
    /// - `super`: Parent module visibility boundary.
    ///
    /// # Returns
    /// The locked registry state.
    ///
    /// # Errors
    /// Returns an internal error when the mutex is poisoned.
    pub(in crate::operation) fn lock(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, OperationRegistryState>, ApplicationError> {
        self.state.lock().map_err(|_| ApplicationError::Internal {
            message: "operation registry lock failed".to_owned(),
        })
    }
}
