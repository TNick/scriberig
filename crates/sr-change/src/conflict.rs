//! Defines structured semantic merge conflicts.

use scriberig_core::{EntityId, RevisionId};

/// Classifies an unresolved universal merge conflict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConflictKind {
    /// Both sides changed an existing entity differently.
    BothModified,
    /// One side removed an entity while the other changed it.
    DeleteModify,
    /// Both sides created the same entity with different revisions.
    CompetingCreate,
    /// Relation membership cannot be merged mechanically.
    RelationConflict,
    /// Collection membership or order is ambiguous.
    CollectionConflict,
    /// A domain-specific interpretation is required.
    DomainConflict,
}

/// One unresolved entity-level merge conflict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MergeConflict {
    /// Entity whose state conflicts.
    pub entity: EntityId,
    /// Base revision, if present.
    pub base: Option<RevisionId>,
    /// Left revision, if present.
    pub left: Option<RevisionId>,
    /// Right revision, if present.
    pub right: Option<RevisionId>,
    /// Conflict classification.
    pub kind: ConflictKind,
    /// Optional stable semantic path within the entity.
    pub path: Option<String>,
    /// Optional domain-provided detail.
    pub detail: Option<String>,
}
