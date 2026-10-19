//! Defines typed collection listing and relation traversal contracts.

use crate::{
    CollectionRef, DetailLevel, EntityInSnapshot, InspectService, NavigationEntry, Page,
    PageRequest, QueryError, RelationRef, SnapshotRef,
};
use scriberig_core::SnapshotId;

/// Requests a bounded page from one snapshot-bound collection.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ListRequest {
    /// Subject owning the collection.
    pub subject: EntityInSnapshot,
    /// Typed collection to enumerate.
    pub collection: CollectionRef,
    /// Page bounds and opaque continuation.
    pub page: PageRequest,
    /// Detail level for navigation entries.
    pub detail: DetailLevel,
}

/// Returns one page of a typed collection.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ListResult {
    /// Subject whose collection was listed.
    pub subject: EntityInSnapshot,
    /// Exact snapshot used by the operation.
    pub snapshot: SnapshotId,
    /// Collection identity preserved in the result.
    pub collection: CollectionRef,
    /// Ordered page returned by the collection provider.
    pub page: Page<NavigationEntry>,
}

/// Selects relation traversal direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum FollowDirection {
    /// Follow relation targets from the source entity.
    Outgoing,
    /// Follow reverse relation targets when an index/provider supports them.
    Incoming,
}

/// Requests a bounded page from one typed relation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FollowRequest {
    /// Subject from which traversal begins.
    pub subject: EntityInSnapshot,
    /// Typed relation to follow.
    pub relation: RelationRef,
    /// Requested traversal direction.
    pub direction: FollowDirection,
    /// Page bounds and opaque continuation.
    pub page: PageRequest,
    /// Detail level for navigation entries.
    pub detail: DetailLevel,
}

/// Returns one page of a typed relation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FollowResult {
    /// Subject from which traversal began.
    pub subject: EntityInSnapshot,
    /// Exact snapshot used by the operation.
    pub snapshot: SnapshotId,
    /// Relation identity preserved in the result.
    pub relation: RelationRef,
    /// Direction used by traversal.
    pub direction: FollowDirection,
    /// Compact relation metadata.
    pub metadata: Option<String>,
    /// Typed navigation entries returned by the provider.
    pub page: Page<NavigationEntry>,
}

/// Provides collection listing and typed relation traversal.
pub trait TraverseService {
    /// Lists one snapshot-bound collection.
    ///
    /// - `self`: Read service implementation.
    /// - `request`: Exact collection listing request.
    ///
    /// One bounded collection page.
    ///
    /// Returns a structured query error when listing is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn list(&self, request: &ListRequest) -> Result<ListResult, QueryError>;

    /// Follows one typed relation in a fixed snapshot.
    ///
    /// - `self`: Read service implementation.
    /// - `request`: Exact relation traversal request.
    ///
    /// One bounded relation page.
    ///
    /// Returns a structured query error when traversal or reverse indexing is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn follow(&self, request: &FollowRequest) -> Result<FollowResult, QueryError>;
}

/// Combines inspection and traversal services into one read boundary.
pub trait ReadService: InspectService + TraverseService {}

impl<T> ReadService for T where T: InspectService + TraverseService {}

/// Keeps snapshot context explicit for APIs that need a standalone reference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotContext(pub SnapshotRef);
