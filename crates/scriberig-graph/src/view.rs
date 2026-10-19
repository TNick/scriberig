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
    /// Lists entity identities visible in one exact snapshot.
    ///
    /// - `self`: Graph view.
    /// - `snapshot`: Exact snapshot context.
    ///
    /// Deterministically ordered entity identities.
    ///
    /// Returns the backend error when enumeration is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn entities(&self, snapshot: SnapshotId) -> Result<Vec<EntityId>, Self::Error>;
    /// Resolves an entity to its active exact revision.
    ///
    /// - `snapshot`: Snapshot context.
    /// - `entity`: Entity identity.
    ///
    /// The active revision, when present.
    ///
    /// Returns the backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, Self::Error>;
    /// Reads semantic state at an exact snapshot and entity.
    ///
    /// - `snapshot`: Snapshot context.
    /// - `entity`: Entity identity.
    ///
    /// The semantic record, when present.
    ///
    /// Returns the backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_revision(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionRecord>, Self::Error>;
    /// Reads an immutable revision without requiring a current snapshot.
    ///
    /// - `entity`: Entity identity associated with the revision.
    /// - `revision`: Exact immutable revision identity.
    ///
    /// The revision record when it is retained and belongs to the entity.
    ///
    /// Returns the backend error when the revision cannot be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_exact_revision(
        &self,
        entity: EntityId,
        revision: RevisionId,
    ) -> Result<Option<RevisionRecord>, Self::Error>;
    /// Returns the declared type for an entity in a snapshot.
    ///
    /// - `snapshot`: Snapshot context.
    /// - `entity`: Entity identity.
    ///
    /// The declared type, when present.
    ///
    /// Returns the backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn entity_type(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<ObjectType>, Self::Error>;
    /// Looks up reverse relations through a view/index abstraction.
    ///
    /// - `snapshot`: Snapshot context.
    /// - `target`: Target entity identity.
    ///
    /// Reverse relation records.
    ///
    /// Returns the backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn reverse_relations(
        &self,
        snapshot: SnapshotId,
        target: EntityId,
    ) -> Result<Vec<ReverseRelation>, Self::Error>;
}
