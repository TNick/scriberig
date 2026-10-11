//! Defines structured protocol errors without transport status codes.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Machine-readable semantic error categories.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum ProtocolErrorCode {
    /// Requested object does not exist.
    NotFound,
    /// Input violates a semantic schema.
    SchemaViolation,
    /// Request used an obsolete base snapshot.
    StaleBaseSnapshot,
    /// Operation conflicts with current semantic state.
    SemanticConflict,
    /// Validation rejected the operation.
    ValidationFailure,
    /// Caller lacks authority.
    PermissionDenied,
    /// Operation is not supported.
    UnsupportedOperation,
    /// Projection is not synchronized.
    ProjectionOutOfSync,
    /// Operation was cancelled.
    OperationCancelled,
    /// Unexpected internal failure.
    Internal,
    /// Client and server protocol majors are incompatible.
    UnsupportedVersion,
}

/// Describes one failed protocol request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProtocolError {
    /// Stable semantic category.
    pub code: ProtocolErrorCode,
    /// Human-readable explanation.
    pub message: String,
    /// Safe structured details for clients.
    pub details: Option<serde_json::Value>,
    /// Correlation identifier when available.
    pub correlation_id: Option<String>,
}
