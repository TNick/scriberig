//! Defines snapshot-aware application inspection requests and results.

use crate::{ApplicationContext, ApplicationError, TrustSummary};
use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};

/// Selects how an application query resolves its snapshot context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotResolution {
    /// Use the application's current accepted snapshot.
    Current,
    /// Resolve the entity under this exact snapshot.
    At(SnapshotId),
    /// Read this exact revision, retaining an optional application snapshot.
    Revision(RevisionId),
}

/// Requests the compact or expanded inspection of one entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectRequest {
    /// Entity identity to inspect.
    pub entity: EntityId,
    /// Snapshot or exact revision resolution requested by the caller.
    pub snapshot: SnapshotResolution,
    /// Amount of semantic detail requested.
    pub detail: DetailLevel,
}

/// Selects the amount of data returned by an inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DetailLevel {
    /// Return orientation metadata and stable follow-up handles.
    Compact,
    /// Include the additional summary fields available from the application.
    Detailed,
}

/// Returns snapshot-bound orientation metadata for an entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectResult {
    /// Entity requested by the caller.
    pub entity: EntityId,
    /// Exact revision resolved by the application, when available.
    pub revision: Option<RevisionId>,
    /// Exact snapshot used by the operation, when available.
    pub snapshot: Option<SnapshotId>,
    /// Semantic type exposed by the graph/schema layer.
    pub object_type: Option<ObjectType>,
    /// Optional short stable label.
    pub label: Option<String>,
    /// Optional short description.
    pub description: Option<String>,
    /// Stable handles for deeper inspection or traversal.
    pub capabilities: Vec<String>,
    /// Trust and provenance summary for this entity.
    pub trust: Option<TrustSummary>,
}

/// Provides shared application-level read operations.
pub trait QueryService {
    /// Inspects one entity under the supplied request context.
    ///
    /// - `self`: Application query service.
    /// - `context`: Caller, capability, and snapshot context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// Snapshot-bound inspection metadata.
    ///
    /// Returns a structured application error when inspection is unavailable or denied.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError>;
}

impl<S: QueryService + ?Sized> QueryService for &S {
    /// Delegates inspection through a borrowed query service.
    ///
    /// - `self`: Borrowed application query service.
    /// - `context`: Caller, capability, and snapshot context.
    /// - `request`: Snapshot-aware inspection request.
    ///
    /// Snapshot-bound inspection metadata.
    ///
    /// Returns the delegated application error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(
        &self,
        context: &ApplicationContext,
        request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        (*self).inspect(context, request)
    }
}
