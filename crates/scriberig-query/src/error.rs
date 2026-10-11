//! Defines structured failures for universal read operations.

use crate::{CollectionRef, RelationRef, TargetRef};
use scriberig_core::{EntityId, SnapshotId};

/// Identifies the read operation that produced an error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadOperation {
    /// Entity inspection.
    Inspect,
    /// Collection listing.
    List,
    /// Relation traversal.
    Follow,
}

/// Reports semantic read failures without exposing storage details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QueryError {
    /// Requested subject or capability does not exist.
    NotFound {
        /// Operation that failed.
        operation: ReadOperation,
        /// Subject involved in the operation.
        target: TargetRef,
    },
    /// Target cannot be resolved for the requested operation.
    InvalidTarget {
        /// Operation that rejected the target.
        operation: ReadOperation,
        /// Invalid target.
        target: TargetRef,
    },
    /// Target has a type incompatible with the requested capability.
    WrongType {
        /// Operation that found the type mismatch.
        operation: ReadOperation,
        /// Entity whose type was incompatible.
        entity: EntityId,
    },
    /// Requested relation, collection, or view is not installed.
    UnsupportedCapability {
        /// Operation that requested the unavailable capability.
        operation: ReadOperation,
        /// Capability name supplied by the caller.
        capability: String,
    },
    /// Exact snapshot cannot be read.
    SnapshotUnavailable {
        /// Operation that required the snapshot.
        operation: ReadOperation,
        /// Snapshot that could not be resolved.
        snapshot: SnapshotId,
    },
    /// A derived index or provider required for traversal is unavailable.
    DerivedDataUnavailable {
        /// Operation that required derived data.
        operation: ReadOperation,
        /// Subject whose derived data is unavailable.
        entity: EntityId,
    },
    /// Continuation does not belong to this operation or snapshot.
    InvalidContinuation {
        /// Operation receiving the invalid continuation.
        operation: ReadOperation,
    },
    /// The query implementation failed without a public semantic category.
    Internal {
        /// Operation that failed.
        operation: ReadOperation,
        /// Safe diagnostic context.
        message: String,
    },
    /// Collection identity was invalid for a list request.
    InvalidCollection {
        /// Operation that rejected the collection.
        operation: ReadOperation,
        /// Collection supplied by the caller.
        collection: CollectionRef,
    },
    /// Relation identity was invalid for a follow request.
    InvalidRelation {
        /// Operation that rejected the relation.
        operation: ReadOperation,
        /// Relation supplied by the caller.
        relation: RelationRef,
    },
}
