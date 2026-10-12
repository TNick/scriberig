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
    /// List a typed collection.
    List,
    /// Follow a typed relation.
    Follow,
    /// Find semantic entities.
    Find,
    /// Compute a semantic diff.
    Diff,
    /// Read semantic history.
    History,
    /// Render a registered projection.
    Render,
    /// Create or update a branch.
    Branch,
    /// Execute a model-driven agent operation.
    Agent,
    /// Apply a change.
    Apply,
    /// Commit a validated change.
    Commit,
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
    /// Stable semantic handles produced by the operation.
    #[serde(default)]
    pub result_handles: Vec<String>,
    /// Structured terminal agent result, when available.
    #[serde(default)]
    pub agent_result: Option<crate::AgentResult>,
    /// Safe failure summary when the operation failed.
    pub error: Option<String>,
}
