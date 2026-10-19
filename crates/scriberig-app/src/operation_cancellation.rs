//! Exposes cooperative operation cancellation through the registry.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{ApplicationError, OperationId, OperationRecord, OperationRegistry, OperationStatus};

impl OperationRegistry {
    /// Cancels one active operation and signals its cooperative executor.
    ///
    /// - `self`: Registry owning operation state.
    /// - `id`: Operation identity to cancel.
    ///
    /// The cancelled operation record.
    ///
    /// Returns a structured failure when cancellation is invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn cancel(&self, id: &OperationId) -> Result<OperationRecord, ApplicationError> {
        let flag = self.cancellation_flag(id)?;
        flag.store(true, Ordering::Release);
        self.transition(id, OperationStatus::Cancelled)
    }

    /// Returns the cooperative cancellation flag for one operation.
    ///
    /// - `self`: Registry owning operation state.
    /// - `id`: Operation identity whose execution is being observed.
    ///
    /// Shared flag set by an application cancellation request.
    ///
    /// Returns `NotFound` when the operation is unknown.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn cancellation_flag(&self, id: &OperationId) -> Result<Arc<AtomicBool>, ApplicationError> {
        let state = self.lock()?;
        state
            .cancellation
            .get(id)
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound {
                subject: id.to_string(),
            })
    }
}
