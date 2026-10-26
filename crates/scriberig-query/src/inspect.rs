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

    /// Inspects one target while checking for branch cancellation.
    ///
    /// Implementations with multiple read stages should check `cancelled`
    /// between stages. The default preserves compatibility and observes
    /// cancellation before and after the synchronous inspection call.
    ///
    /// - `self`: Read service implementation.
    /// - `request`: Exact-target inspection request.
    /// - `cancelled`: Shared branch cancellation flag.
    ///
    /// Exact inspection metadata unless cancellation was requested.
    ///
    /// # Returns
    /// The semantic inspection result.
    ///
    /// # Errors
    /// Returns `Cancelled` when cancellation is observed, or the query error
    /// returned by `inspect`.
    fn inspect_cancellable(
        &self,
        request: &InspectRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Inspection, QueryError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(QueryError::Cancelled {
                operation: crate::ReadOperation::Inspect,
            });
        }
        let result = self.inspect(request)?;
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(QueryError::Cancelled {
                operation: crate::ReadOperation::Inspect,
            });
        }
        Ok(result)
    }
}
