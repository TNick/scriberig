//! Defines serializable application event envelopes.

use crate::refs::{EntityReference, OperationReference, RevisionReference, SnapshotReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stable categories of public runtime events.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum EventKind {
    /// Canonical snapshot changed.
    SnapshotChanged,
    /// Entity revision changed.
    EntityChanged,
    /// Projection changed.
    ProjectionChanged,
    /// Branch changed.
    BranchChanged,
    /// Session state changed and should be refreshed.
    SessionChanged,
    /// Safe session model or thinking selection activity was emitted.
    SessionSelectionChanged,
    /// Migration was created.
    MigrationCreated,
    /// Operation started.
    OperationStarted,
    /// Operation progress changed.
    OperationProgressed,
    /// Operation completed.
    OperationCompleted,
    /// Operation failed.
    OperationFailed,
    /// Operation was cancelled.
    OperationCancelled,
    /// Validation completed.
    ValidationCompleted,
    /// Agent output was produced.
    AgentOutputProduced,
    /// Incremental transient agent output text.
    AgentOutputDelta,
    /// Transient safe agent status activity.
    AgentActivity,
    /// A candidate snapshot was created.
    CandidateCreated,
    /// Validation started for a candidate or operation.
    ValidationStarted,
    /// An approval was recorded.
    ApprovalRecorded,
    /// A semantic conflict was detected.
    ConflictDetected,
    /// A capability check denied an action.
    CapabilityDenied,
    /// An external effect receipt changed state.
    ExternalEffectUpdated,
    /// A candidate snapshot was promoted.
    SnapshotPromoted,
}

/// Carries stable references and concise metadata for one event.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Event {
    /// Monotonic protocol event cursor within the server event history.
    #[serde(default)]
    pub sequence: u64,
    /// Event category.
    pub kind: EventKind,
    /// Optional affected entity.
    pub entity: Option<EntityReference>,
    /// Optional affected revision.
    pub revision: Option<RevisionReference>,
    /// Optional affected snapshot.
    pub snapshot: Option<SnapshotReference>,
    /// Optional operation handle.
    pub operation: Option<OperationReference>,
    /// Concise metadata safe for all clients.
    pub metadata: Vec<(String, String)>,
}
