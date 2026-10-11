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
    /// Migration was created.
    MigrationCreated,
    /// Operation started.
    OperationStarted,
    /// Operation progress changed.
    OperationProgressed,
    /// Operation completed.
    OperationCompleted,
    /// Validation completed.
    ValidationCompleted,
    /// Agent output was produced.
    AgentOutputProduced,
}

/// Carries stable references and concise metadata for one event.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Event {
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
