//! Implements integrated agent execution and orchestration above reusable agent interfaces.

use scriberig_app::{
    ApplicationContext, ApplicationError, InspectRequest, InspectResult, QueryService,
};

/// Query boundary used by the executable until a local persistence backend is configured.
pub struct UnavailableQueryService;

impl QueryService for UnavailableQueryService {
    /// Reports that local application persistence is not configured.
    ///
    /// # Parameters
    /// - `self`: Unavailable service marker.
    /// - `_context`: Application request context.
    /// - `_request`: Snapshot-aware inspection request.
    ///
    /// # Returns
    /// Never returns a successful result.
    ///
    /// # Errors
    /// Always reports an unavailable application operation.
    fn inspect(
        &self,
        _context: &ApplicationContext,
        _request: &InspectRequest,
    ) -> Result<InspectResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "inspect without configured local application".to_owned(),
        })
    }
}
