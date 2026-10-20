//! Implements lifecycle construction and transition for operation records.

use super::{OperationId, OperationKind, OperationRecord, OperationStatus};

impl OperationRecord {
    /// Creates a queued operation record.
    ///
    /// - `reference`: Stable operation identity.
    /// - `kind`: Universal operation family.
    ///
    /// A queued operation record without mutable result data.
    #[must_use]
    /// # Returns
    /// The operation result.
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
    /// - `self`: Operation record to update.
    /// - `status`: Next lifecycle status.
    ///
    /// The updated operation record.
    ///
    /// Returns an application error when a terminal operation is changed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn transition(mut self, status: OperationStatus) -> Result<Self, crate::ApplicationError> {
        if self.status.is_terminal() {
            return Err(crate::ApplicationError::OperationFailed {
                operation: self.reference.to_string(),
                message: "terminal operation cannot transition".to_owned(),
            });
        }
        self.status = status;
        if status.is_terminal()
            && let Some(progress) = self.progress.as_mut()
        {
            progress.status = status;
        }
        Ok(self)
    }
}
