//! Defines structured application-operation failures.

use scriberig_core::{EntityId, RevisionId, SnapshotId};
use std::fmt::{self, Display, Formatter};

/// Reports a semantic application failure without transport-specific details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationError {
    /// Requested semantic data does not exist.
    NotFound {
        /// Stable subject that could not be found.
        subject: String,
    },
    /// Input violates a domain or schema rule.
    SchemaViolation {
        /// Safe schema diagnostic.
        message: String,
    },
    /// The operation was based on an obsolete snapshot.
    StaleBaseSnapshot {
        /// Snapshot supplied by the caller.
        requested: SnapshotId,
        /// Snapshot currently accepted by the application.
        current: SnapshotId,
    },
    /// The requested semantic merge or reconciliation is ambiguous.
    SemanticConflict {
        /// Stable subject involved in the conflict.
        subject: String,
    },
    /// Validation produced a failure for the requested candidate.
    ValidationFailure {
        /// Safe validation diagnostic.
        message: String,
    },
    /// The caller lacks authority for the requested operation.
    PermissionDenied {
        /// Requested action.
        action: String,
        /// Requested authority scope.
        scope: String,
    },
    /// The operation is not supported by this application configuration.
    UnsupportedOperation {
        /// Stable unsupported operation name.
        operation: String,
    },
    /// A projection is not synchronized with the requested semantic state.
    ProjectionOutOfSync {
        /// Stable projection subject.
        subject: String,
    },
    /// A long-running operation was cancelled.
    OperationCancelled {
        /// Stable operation name.
        operation: String,
    },
    /// An implementation failure with a safe, backend-neutral message.
    Internal {
        /// Safe implementation diagnostic.
        message: String,
    },
    /// The requested revision does not belong to the requested entity.
    RevisionMismatch {
        /// Entity identity supplied by the caller.
        entity: EntityId,
        /// Revision identity supplied by the caller.
        revision: RevisionId,
    },
}

impl ApplicationError {
    /// Returns the stable category name used by adapters.
    ///
    /// # Parameters
    /// - `self`: Error to classify.
    ///
    /// # Returns
    /// A stable category label.
    #[must_use]
    pub const fn category(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "not_found",
            Self::SchemaViolation { .. } => "schema_violation",
            Self::StaleBaseSnapshot { .. } => "stale_base_snapshot",
            Self::SemanticConflict { .. } => "semantic_conflict",
            Self::ValidationFailure { .. } => "validation_failure",
            Self::PermissionDenied { .. } => "permission_denied",
            Self::UnsupportedOperation { .. } => "unsupported_operation",
            Self::ProjectionOutOfSync { .. } => "projection_out_of_sync",
            Self::OperationCancelled { .. } => "operation_cancelled",
            Self::Internal { .. } => "internal",
            Self::RevisionMismatch { .. } => "revision_mismatch",
        }
    }
}

impl Display for ApplicationError {
    /// Formats an application error for a human-facing adapter.
    ///
    /// # Parameters
    /// - `self`: Error to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects the write.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { subject } => write!(formatter, "subject not found: {subject}"),
            Self::SchemaViolation { message } => {
                write!(formatter, "schema violation: {message}")
            }
            Self::StaleBaseSnapshot { requested, current } => {
                write!(
                    formatter,
                    "stale snapshot {requested}; current is {current}"
                )
            }
            Self::SemanticConflict { subject } => {
                write!(formatter, "semantic conflict: {subject}")
            }
            Self::ValidationFailure { message } => {
                write!(formatter, "validation failed: {message}")
            }
            Self::PermissionDenied { action, scope } => {
                write!(formatter, "permission denied for {action} in {scope}")
            }
            Self::UnsupportedOperation { operation } => {
                write!(formatter, "unsupported operation: {operation}")
            }
            Self::ProjectionOutOfSync { subject } => {
                write!(formatter, "projection out of sync: {subject}")
            }
            Self::OperationCancelled { operation } => {
                write!(formatter, "operation cancelled: {operation}")
            }
            Self::Internal { message } => {
                write!(formatter, "internal application error: {message}")
            }
            Self::RevisionMismatch { entity, revision } => {
                write!(
                    formatter,
                    "revision {revision} does not belong to entity {entity}"
                )
            }
        }
    }
}

impl std::error::Error for ApplicationError {}
