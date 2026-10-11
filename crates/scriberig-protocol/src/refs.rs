//! Defines transport-safe semantic references.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// References one semantic entity by canonical textual identity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct EntityReference {
    /// Canonical entity identifier.
    pub id: String,
}

/// References one immutable entity revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct RevisionReference {
    /// Entity containing the revision.
    pub entity: EntityReference,
    /// Canonical revision identifier.
    pub id: String,
}

/// References one immutable snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SnapshotReference {
    /// Canonical snapshot identifier.
    pub id: String,
}

/// References one long-running operation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct OperationReference {
    /// Canonical operation identifier.
    pub id: String,
}
