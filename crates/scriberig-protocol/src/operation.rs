//! Defines serializable long-running operation state.

use crate::refs::{OperationReference, SnapshotReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Public operation categories.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum OperationKind {
    /// Inspect semantic state.
    Inspect,
    /// Execute a query.
    Query,
    /// Apply a change.
    Apply,
    /// Validate a candidate.
    Validate,
    /// Promote a candidate.
    Promote,
    /// Extensible operation category.
    Custom(String),
}

/// Terminal and active operation states.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum OperationStatus {
    /// Accepted but not started.
    Queued,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with an error.
    Failed,
    /// Explicitly cancelled.
    Cancelled,
}

/// Reports operation progress as a bounded percentage and message.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct OperationProgress {
    /// Percentage from zero through one hundred.
    pub percent: u8,
    /// Optional concise progress description.
    pub message: Option<String>,
}

/// Represents the public operation lifecycle.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Operation {
    /// Stable operation handle.
    pub reference: OperationReference,
    /// Semantic operation category.
    pub kind: OperationKind,
    /// Current lifecycle state.
    pub status: OperationStatus,
    /// Latest progress observation.
    pub progress: Option<OperationProgress>,
    /// Result snapshot when the operation completed.
    pub result_snapshot: Option<SnapshotReference>,
    /// Safe failure summary when the operation failed.
    pub error: Option<String>,
}
