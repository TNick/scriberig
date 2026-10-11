//! Defines stable long-running application operation state.

pub use scriberig_core::OperationId;

/// Classifies the semantic work performed by an operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKind {
    /// A read-only inspection or query.
    Query,
    /// A validation operation.
    Validation,
    /// A promotion or canonical-state transition.
    Promotion,
    /// An external effect or synchronization operation.
    ExternalEffect,
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
