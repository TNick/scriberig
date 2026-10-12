//! Defines structured application-operation failures.

use scriberig_core::{EntityId, RevisionId, SnapshotId};
use std::fmt::{self, Display, Formatter};

/// Reports a semantic application failure without transport-specific details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationError {
    /// Request parameters are inconsistent or incomplete.
    InvalidRequest {
        /// Universal operation being requested.
        operation: String,
        /// Safe request diagnostic.
        message: String,
    },
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
    /// A remote transport could not be reached or completed its request.
    TransportUnavailable {
        /// Safe transport diagnostic.
        message: String,
    },
    /// A remote protocol returned a semantic failure category.
    ProtocolFailure {
        /// Stable protocol error category.
        category: String,
        /// Safe protocol diagnostic.
        message: String,
    },
    /// The requested revision does not belong to the requested entity.
    RevisionMismatch {
        /// Entity identity supplied by the caller.
        entity: EntityId,
        /// Revision identity supplied by the caller.
        revision: RevisionId,
    },
    /// A requested capability is not installed or available.
    UnsupportedCapability {
        /// Universal operation requesting the capability.
        operation: String,
        /// Stable capability identifier.
        capability: String,
    },
    /// The requested state transition has competing semantic candidates.
    Conflict {
        /// Stable subject involved in the conflict.
        subject: String,
        /// Structured conflict detail.
        details: Box<ConflictDetails>,
    },
    /// Multiple semantic candidates remain possible.
    Ambiguous {
        /// Scope in which candidates were found.
        scope: String,
        /// Stable candidate handles.
        candidates: Vec<String>,
        /// Safe reason for the ambiguity.
        reason: String,
    },
    /// The requested exact snapshot is unavailable.
    SnapshotUnavailable {
        /// Snapshot required by the operation.
        snapshot: SnapshotId,
    },
    /// A tracked operation failed with structured context.
    OperationFailed {
        /// Stable operation identity or kind.
        operation: String,
        /// Safe failure diagnostic.
        message: String,
    },
}

/// Preserves semantic handles associated with a conflict.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConflictDetails {
    /// Base revision used by the attempted operation.
    pub base_revision: Option<RevisionId>,
    /// Revision expected by the operation.
    pub expected_revision: Option<RevisionId>,
    /// Revision currently present.
    pub current_revision: Option<RevisionId>,
    /// Competing left revision or migration handle.
    pub left: Option<String>,
    /// Competing right revision or migration handle.
    pub right: Option<String>,
    /// Relation or collection operation involved in the conflict.
    pub operation: Option<String>,
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
    pub fn category(&self) -> &str {
        match self {
            Self::InvalidRequest { .. } => "invalid_request",
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
            Self::TransportUnavailable { .. } => "transport_unavailable",
            Self::ProtocolFailure { category, .. } => category.as_str(),
            Self::RevisionMismatch { .. } => "revision_mismatch",
            Self::UnsupportedCapability { .. } => "unsupported_capability",
            Self::Conflict { .. } => "conflict",
            Self::Ambiguous { .. } => "ambiguous",
            Self::SnapshotUnavailable { .. } => "snapshot_unavailable",
            Self::OperationFailed { .. } => "operation_failed",
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
            Self::InvalidRequest { operation, message } => {
                write!(formatter, "invalid {operation} request: {message}")
            }
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
            Self::TransportUnavailable { message } => {
                write!(formatter, "transport unavailable: {message}")
            }
            Self::ProtocolFailure { category, message } => {
                write!(formatter, "{category}: {message}")
            }
            Self::RevisionMismatch { entity, revision } => {
                write!(
                    formatter,
                    "revision {revision} does not belong to entity {entity}"
                )
            }
            Self::UnsupportedCapability {
                operation,
                capability,
            } => write!(
                formatter,
                "unsupported capability {capability} for {operation}"
            ),
            Self::Conflict { subject, .. } => write!(formatter, "conflict for subject {subject}"),
            Self::Ambiguous { scope, .. } => write!(formatter, "ambiguous result in {scope}"),
            Self::SnapshotUnavailable { snapshot } => {
                write!(formatter, "snapshot unavailable: {snapshot}")
            }
            Self::OperationFailed { operation, message } => {
                write!(formatter, "operation {operation} failed: {message}")
            }
        }
    }
}

impl std::error::Error for ApplicationError {}
