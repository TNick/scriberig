//! Defines work requested by the reducer but executed by the runtime.

/// A presentation or future semantic effect emitted by state reduction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    /// No work is required outside the reducer.
    None,
    /// Ask the owning service adapter to refresh presentation data.
    RequestRefresh,
    /// Ask the owning service adapter to execute a find request.
    RequestFind {
        /// Query text supplied by the user.
        text: String,
    },
    /// Ask the owning service adapter to execute a contextual action later.
    RequestSemanticAction {
        /// Stable semantic action identifier.
        id: String,
        /// Explicit action inputs.
        inputs: Vec<(String, String)>,
    },
    /// Request runtime termination.
    RequestQuit,
    /// Request semantic inspection through the TUI service port.
    InspectEntity {
        /// Monotonic request identifier.
        request_id: u64,
        /// Entity to inspect.
        entity: scriberig_core::EntityId,
        /// Optional exact snapshot.
        snapshot: Option<scriberig_core::SnapshotId>,
    },
    /// Request the current runtime capability descriptors.
    RequestCapabilities,
    /// Read exact semantic history.
    History {
        /// Request generation.
        request_id: u64,
        /// Exact snapshot context.
        snapshot: scriberig_core::SnapshotId,
    },
    /// Compute a structured two-sided semantic diff.
    Diff {
        /// Request generation.
        request_id: u64,
        /// Public exact diff request.
        request: scriberig_app::DiffRequest,
    },
    /// Render through a shared projection operation.
    Render {
        /// Request generation.
        request_id: u64,
        /// Public render request.
        request: scriberig_app::RenderRequest,
    },
    /// Apply a candidate change without promotion.
    Apply {
        /// Request generation.
        request_id: u64,
        /// Public apply request.
        request: scriberig_app::ApplyRequest,
    },
    /// Commit a reviewed change with its exact base context.
    Commit {
        /// Request generation.
        request_id: u64,
        /// Public commit request.
        request: scriberig_app::CommitRequest,
    },
    /// Read the authoritative lifecycle record for one operation.
    OperationStatus {
        /// Operation identity to inspect.
        operation: scriberig_core::OperationId,
    },
    /// Ask the application to cancel one operation.
    CancelOperation {
        /// Operation identity to cancel.
        operation: scriberig_core::OperationId,
    },
}
