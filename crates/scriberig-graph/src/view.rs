//! Defines snapshot-bound graph reads and derived reverse navigation.

use crate::RevisionRecord;
use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};

/// A reverse relation discovered through an index or view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReverseRelation {
    /// Source semantic entity.
    pub source: EntityId,
    /// Relation type used by the source.
    pub relation_type: ObjectType,
    /// Source revision resolved in the requested snapshot, when available.
    pub source_revision: Option<RevisionId>,
}

/// Read-only graph contract whose results are explicitly snapshot-bound.
pub trait KnowledgeView {
    /// Backend-specific failure type.
    type Error;
    /// Resolves an entity to its active exact revision.
    ///
    /// # Parameters
    /// - `snapshot`: Snapshot context.
    /// - `entity`: Entity identity.
    ///
    /// # Returns
    /// The active revision, when present.
    ///
    /// # Errors
    /// Returns the backend error.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, Self::Error>;
    /// Reads semantic state at an exact snapshot and entity.
    ///
    /// # Parameters
    /// - `snapshot`: Snapshot context.
    /// - `entity`: Entity identity.
    ///
    /// # Returns
    /// The semantic record, when present.
    ///
    /// # Errors
    /// Returns the backend error.
    fn read_revision(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionRecord>, Self::Error>;
    /// Returns the declared type for an entity in a snapshot.
    ///
    /// # Parameters
    /// - `snapshot`: Snapshot context.
    /// - `entity`: Entity identity.
    ///
    /// # Returns
    /// The declared type, when present.
    ///
    /// # Errors
    /// Returns the backend error.
    fn entity_type(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<ObjectType>, Self::Error>;
    /// Looks up reverse relations through a view/index abstraction.
    ///
    /// # Parameters
    /// - `snapshot`: Snapshot context.
    /// - `target`: Target entity identity.
    ///
    /// # Returns
    /// Reverse relation records.
    ///
    /// # Errors
    /// Returns the backend error.
    fn reverse_relations(
        &self,
        snapshot: SnapshotId,
        target: EntityId,
    ) -> Result<Vec<ReverseRelation>, Self::Error>;
}
