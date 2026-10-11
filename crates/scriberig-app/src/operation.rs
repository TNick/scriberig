//! Defines stable long-running application operation state.

pub use scriberig_core::OperationId;

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
