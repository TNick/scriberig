//! Adapts canonical projection adapters to universal render operations.

use crate::{
    ApplicationContext, ApplicationError, ApplyRequest, BranchRequest, CommitRequest, DiffRequest,
    HistoryRequest, OperationProvider, OperationSummary, RenderRequest,
};
use sr_projection::{ProjectionAdapter, RenderRequest as ProjectionRequest};

/// Executes render through a canonical projection adapter and request factory.
pub struct ProjectionOperationProvider<P, F> {
    adapter: P,
    request_factory: F,
}

impl<P, F> ProjectionOperationProvider<P, F> {
    /// Creates a projection operation provider.
    ///
    /// # Parameters
    /// - `adapter`: Canonical projection adapter that owns rendering.
    /// - `request_factory`: Converts the universal request to typed projection inputs.
    ///
    /// # Returns
    /// A provider that delegates render without duplicating projection logic.
    #[must_use]
    pub const fn new(adapter: P, request_factory: F) -> Self {
        Self {
            adapter,
            request_factory,
        }
    }
}

impl<P, F> OperationProvider for ProjectionOperationProvider<P, F>
where
    P: ProjectionAdapter,
    F: Fn(&RenderRequest) -> Result<ProjectionRequest, ApplicationError>,
{
    /// Reports that diff ownership is not configured by this provider.
    ///
    /// # Parameters
    /// - `self`: Projection provider.
    /// - `_context`: Application context.
    /// - `_request`: Diff request.
    ///
    /// # Returns
    /// Never returns a diff summary.
    ///
    /// # Errors
    /// Returns unsupported-operation.
    fn diff(
        &self,
        _context: &ApplicationContext,
        _request: &DiffRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("diff"))
    }

    /// Reports that history ownership is not configured by this provider.
    ///
    /// # Parameters
    /// - `self`: Projection provider.
    /// - `_context`: Application context.
    /// - `_request`: History request.
    ///
    /// # Returns
    /// Never returns a history summary.
    ///
    /// # Errors
    /// Returns unsupported-operation.
    fn history(
        &self,
        _context: &ApplicationContext,
        _request: &HistoryRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("history"))
    }

    /// Delegates rendering to the canonical projection adapter.
    ///
    /// # Parameters
    /// - `self`: Projection provider.
    /// - `_context`: Application context.
    /// - `request`: Universal render request.
    ///
    /// # Returns
    /// Render result summary with output hash and exact snapshot.
    ///
    /// # Errors
    /// Returns a structured projection or factory failure.
    fn render(
        &self,
        _context: &ApplicationContext,
        request: &RenderRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        let projection_request = (self.request_factory)(request)?;
        let rendered = self
            .adapter
            .render(&projection_request)
            .map_err(|diagnostic| ApplicationError::Internal {
                message: format!("projection render failed: {diagnostic:?}"),
            })?;
        let handle = rendered.output_hash.as_ref().map_or_else(
            || "render:unhashed".to_owned(),
            |hash| format!("{}:{:x?}", hash.algorithm, hash.digest),
        );
        Ok(OperationSummary {
            handle,
            snapshot: Some(rendered.base_state.0),
            revision: None,
            branch: None,
            diff: None,
            history: None,
            rendered: Some(rendered),
            validation: None,
            validation_diagnostics: Vec::new(),
            conflicts: Vec::new(),
        })
    }

    /// Reports that branch ownership is not configured by this provider.
    ///
    /// # Parameters
    /// - `self`: Projection provider.
    /// - `_context`: Application context.
    /// - `_request`: Branch request.
    ///
    /// # Returns
    /// Never returns a branch summary.
    ///
    /// # Errors
    /// Returns unsupported-operation.
    fn branch(
        &self,
        _context: &ApplicationContext,
        _request: &BranchRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("branch"))
    }

    /// Reports that change ownership is not configured by this provider.
    ///
    /// # Parameters
    /// - `self`: Projection provider.
    /// - `_context`: Application context.
    /// - `_request`: Apply request.
    ///
    /// # Returns
    /// Never returns a candidate summary.
    ///
    /// # Errors
    /// Returns unsupported-operation.
    fn apply(
        &self,
        _context: &ApplicationContext,
        _request: &ApplyRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("apply"))
    }

    /// Reports that promotion ownership is not configured by this provider.
    ///
    /// # Parameters
    /// - `self`: Projection provider.
    /// - `_context`: Application context.
    /// - `_request`: Commit request.
    ///
    /// # Returns
    /// Never returns a promotion summary.
    ///
    /// # Errors
    /// Returns unsupported-operation.
    fn commit(
        &self,
        _context: &ApplicationContext,
        _request: &CommitRequest,
    ) -> Result<OperationSummary, ApplicationError> {
        Err(unsupported("commit"))
    }
}

/// Creates a stable unsupported-operation error.
///
/// # Parameters
/// - `operation`: Operation name without a configured provider.
///
/// # Returns
/// Structured unsupported-operation failure.
fn unsupported(operation: &str) -> ApplicationError {
    ApplicationError::UnsupportedOperation {
        operation: operation.to_owned(),
    }
}
