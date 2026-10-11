//! Defines generic projection lifecycle status independent of storage location.

/// Describes the current synchronization or import state of a projection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProjectionStatus {
    /// External representation matches the last successful render.
    Synchronized,
    /// External representation differs from its recorded render.
    ExternallyModified,
    /// A render operation is in progress.
    Rendering,
    /// Parsed external changes await semantic acceptance.
    ImportPending,
    /// Import failed before canonical state could be changed.
    ImportFailed,
    /// Reconciliation requires conflict resolution.
    Conflict,
    /// The external representation is unavailable.
    Missing,
}
