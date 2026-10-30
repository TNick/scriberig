//! Defines storage change, diff, and error values for immutable snapshots.

use scriberig_core::{EncodingError, EntityId, RevisionId, SnapshotId};

/// Describes one storage-level snapshot update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotChange {
    /// Associates an entity with a revision.
    Set(EntityId, RevisionId),
    /// Removes an entity association.
    Remove(EntityId),
}

/// Identifies the source change that produced one immutable snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotChangeProvenance {
    /// Stable identity of the accepted source change.
    pub change_id: String,
    /// Stable semantic operation recorded by the candidate, when known.
    pub semantic_operation: Option<String>,
}

/// Describes one semantic difference between two immutable snapshots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotDiff {
    /// An entity is present only in the target snapshot.
    Added {
        /// Entity introduced by the target snapshot.
        entity: EntityId,
        /// Revision assigned by the target snapshot.
        revision: RevisionId,
    },
    /// An entity is present only in the source snapshot.
    Removed {
        /// Entity removed by the target snapshot.
        entity: EntityId,
        /// Revision assigned by the source snapshot.
        revision: RevisionId,
    },
    /// An entity resolves to a different revision in the target snapshot.
    Changed {
        /// Entity whose resolution changed.
        entity: EntityId,
        /// Revision assigned by the source snapshot.
        from: RevisionId,
        /// Revision assigned by the target snapshot.
        to: RevisionId,
    },
}

/// Reports an invalid snapshot operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    /// The requested canonical encoding version is not supported.
    InvalidEncoding(EncodingError),
    /// The requested snapshot is not known by the backend.
    UnknownSnapshot(SnapshotId),
    /// A backend-specific durable storage failure.
    Backend(String),
}
