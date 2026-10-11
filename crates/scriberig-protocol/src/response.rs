//! Defines presentation-neutral protocol responses.

use crate::operation::Operation;
use crate::refs::{EntityReference, RevisionReference, SnapshotReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Returns the semantic result of entity inspection.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct InspectEntityResponse {
    /// Inspected entity.
    pub entity: EntityReference,
    /// Exact revision selected by resolution.
    pub revision: RevisionReference,
    /// Exact snapshot used for resolution.
    pub snapshot: SnapshotReference,
    /// Stable semantic object type.
    pub object_type: Option<String>,
    /// Presentation-neutral semantic summary.
    pub summary: Option<String>,
}

/// Returns the current operation lifecycle state.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct OperationStatusResponse {
    /// Operation state and optional result summary.
    pub operation: Operation,
}

/// Returns one structured query page without presentation formatting.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QueryResponse {
    /// Exact query result and snapshot scope.
    pub result: serde_json::Value,
}
