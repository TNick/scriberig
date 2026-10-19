//! Defines semantic requests independent of transport endpoints.

use crate::refs::{EntityReference, OperationReference, SnapshotReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Selects the snapshot against which an inspection resolves.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SnapshotSelector {
    /// Resolve against an exact immutable snapshot.
    Exact(SnapshotReference),
    /// Resolve against a named root selected by the server.
    NamedRoot(String),
    /// Resolve against the current canonical root.
    Current,
}

/// Requests semantic inspection of one entity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct InspectEntityRequest {
    /// Entity to inspect.
    pub entity: EntityReference,
    /// Snapshot resolution rule.
    pub snapshot: SnapshotSelector,
    /// Requested presentation-neutral detail level.
    pub detail: DetailLevel,
}

/// Selects the amount of semantic summary returned.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum DetailLevel {
    /// Return identifiers and a compact summary.
    Compact,
    /// Return all public semantic summary fields.
    Deep,
}

/// Requests the runtime capability vocabulary.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct RuntimeCapabilitiesRequest {}

/// Requests the current status of an operation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct OperationStatusRequest {
    /// Operation to inspect.
    pub operation: OperationReference,
}

/// Requests cancellation of an operation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CancelOperationRequest {
    /// Operation to cancel.
    pub operation: OperationReference,
}

/// Requests a subscription to selected event categories.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SubscribeRequest {
    /// Optional event names; empty means all public events.
    pub event_kinds: Vec<String>,
    /// Return only events published after this cursor.
    #[serde(default)]
    pub after: Option<u64>,
}

/// Requests one exact structural query through the shared query model.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct QueryRequest {
    /// Snapshot-bound structural query specification.
    pub specification: serde_json::Value,
}

/// Requests one bounded semantic collection page.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ListRequest {
    /// Serialized universal list request retained by the protocol boundary.
    pub specification: serde_json::Value,
}

/// Requests bounded semantic discovery within an exact scope.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct FindRequest {
    /// Serialized universal find request retained by the protocol boundary.
    pub specification: serde_json::Value,
}

/// Requests one semantic command using a stable application operation kind.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CommandRequest {
    /// Command family understood by the application boundary.
    pub kind: String,
    /// JSON representation of the typed command payload.
    pub payload: serde_json::Value,
}

/// Requests one registered domain operation on an exact semantic selection.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct DomainOperationCommand {
    /// Registered capability name.
    pub capability: String,
    /// Selected semantic entity.
    pub entity: EntityReference,
    /// Exact snapshot containing the selection.
    pub snapshot: SnapshotReference,
    /// Optional exact selected revision.
    pub revision: Option<String>,
    /// Optional named root branch.
    pub branch: Option<String>,
    /// Typed scalar arguments encoded with the query value vocabulary.
    pub arguments: std::collections::BTreeMap<String, serde_json::Value>,
}

/// Requests one project onboarding operation through the public protocol.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ProjectRequest {
    /// Stable project operation name such as `scan` or `import`.
    pub operation: String,
    /// Operation-specific JSON payload.
    pub payload: serde_json::Value,
}

/// Requests the credential-safe model library.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelLibraryRequest {}

/// Requests a session model-profile selection mutation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionModelSelectionRequest {
    /// Stable session identity.
    pub session: crate::SessionReference,
    /// Mutation revision observed by the caller.
    pub expected_revision: u64,
    /// Curated model profile to select.
    pub model_profile: String,
}

/// Requests a session thinking-level mutation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionThinkingSelectionRequest {
    /// Stable session identity.
    pub session: crate::SessionReference,
    /// Mutation revision observed by the caller.
    pub expected_revision: u64,
    /// Advertised thinking id to select.
    pub thinking_level: String,
}
