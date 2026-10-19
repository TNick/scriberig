//! Defines exact-target entity inspection requests.

use crate::{DetailLevel, Inspection, ProjectionRef, QueryError, TargetRef};

/// Requests inspection of one exact semantic target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectRequest {
    /// Snapshot-relative or exact-revision target.
    pub target: TargetRef,
    /// Amount of orientation detail requested.
    pub detail: DetailLevel,
    /// Optional registered view requested by the caller.
    pub view: Option<ProjectionRef>,
}

/// Provides exact-target entity inspection.
pub trait InspectService {
    /// Inspects one target without changing its snapshot or revision context.
    ///
    /// - `self`: Read service implementation.
    /// - `request`: Exact-target inspection request.
    ///
    /// Compact or expanded inspection metadata.
    ///
    /// Returns a structured query error when the target or view is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(&self, request: &InspectRequest) -> Result<Inspection, QueryError>;
}
