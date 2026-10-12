//! Implements lifecycle construction and transition for operation records.

use super::{OperationId, OperationKind, OperationProgress, OperationRecord, OperationStatus};

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
            agent_result: None,
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
