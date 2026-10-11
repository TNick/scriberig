//! Defines transport-independent application events with stable handles.

use crate::operation::{OperationId, OperationProgress};
use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Reports a semantic state transition or operation update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationEvent {
    /// The accepted snapshot root changed.
    SnapshotChanged {
        /// New accepted snapshot identity.
        snapshot: SnapshotId,
    },
    /// An entity revision changed.
    EntityChanged {
        /// Stable entity identity.
        entity: EntityId,
        /// New exact revision identity.
        revision: RevisionId,
    },
    /// A projection changed state.
    ProjectionChanged {
        /// Stable projection subject.
        subject: EntityId,
    },
    /// A branch changed state.
    BranchChanged {
        /// Stable branch identity.
        branch: String,
    },
    /// A migration was created.
    MigrationCreated {
        /// Stable migration identity.
        migration: String,
    },
    /// An operation started.
    OperationStarted {
        /// Stable operation identity.
        operation: OperationId,
    },
    /// An operation reported progress.
    OperationProgressed {
        /// Operation whose progress changed.
        operation: OperationId,
        /// Concise progress state.
        progress: OperationProgress,
    },
    /// An operation completed.
    OperationCompleted {
        /// Stable operation identity.
        operation: OperationId,
    },
    /// Validation completed for a stable subject handle.
    ValidationCompleted {
        /// Stable validated subject identity.
        subject: String,
        /// Whether the validation passed.
        passed: bool,
    },
    /// An agent produced output identified by a stable handle.
    AgentOutputProduced {
        /// Stable output identity.
        output: String,
    },
}
