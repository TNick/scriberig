//! Defines opaque semantic handles used by universal queries.

use scriberig_core::{EntityId, OperationId, RevisionId, SnapshotId};

/// References one semantic entity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct EntityRef(pub EntityId);

/// References one exact entity revision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RevisionRef(pub RevisionId);

/// References one exact canonical snapshot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SnapshotRef(pub SnapshotId);

/// Identifies a typed relation without exposing storage coordinates.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RelationRef {
    /// Source entity of the relation.
    pub source: EntityId,
    /// Stable semantic relation type.
    pub relation_type: String,
    /// Target entity when the relation resolves to one entity.
    pub target: Option<EntityId>,
}

/// Identifies a typed collection owned by an entity.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CollectionRef {
    /// Owning entity of the collection.
    pub owner: EntityId,
    /// Stable semantic collection type.
    pub collection_type: String,
}

/// Identifies a projection independently of its physical path or UI.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ProjectionRef(pub String);

/// Identifies a tracked application operation.
#[derive(Clone, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OperationRef(pub OperationId);

/// Selects an entity at one exact revision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct EntityAtRevision {
    /// Entity identity.
    pub entity: EntityId,
    /// Exact revision identity.
    pub revision: RevisionId,
}

/// Selects an entity relative to one exact snapshot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct EntityInSnapshot {
    /// Entity identity.
    pub entity: EntityId,
    /// Exact snapshot context.
    pub snapshot: SnapshotId,
}

/// Identifies a query target while preserving revision/snapshot semantics.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum TargetRef {
    /// Exact entity revision target.
    Revision(EntityAtRevision),
    /// Snapshot-relative entity target.
    Snapshot(EntityInSnapshot),
}
