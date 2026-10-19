//! Defines stable application operation classifications.

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
    /// Coordinates durable plan advancement and worker dispatch.
    Plan,
    /// Applies a proposed change.
    Apply,
    /// Commits a validated change.
    Commit,
    /// Produces an exact-base change through a domain operation.
    DomainOperation,
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
    /// - `self`: Status to classify.
    ///
    /// `true` for completed, failed, and cancelled states.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}
