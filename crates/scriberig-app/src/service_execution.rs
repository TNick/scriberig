//! Applies render style resolution and cancellation to universal requests.

use super::ApplicationService;
use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult,
};

impl<B: ApplicationBackend> ApplicationService<B> {
    /// Executes one universal application request.
    ///
    /// - `self`: Application service.
    /// - `context`: Caller and capability context.
    /// - `request`: Universal request to execute.
    ///
    /// The structured result returned by the owning subsystem.
    ///
    /// Returns a structured application failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn execute(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        match request {
            ApplicationRequest::Render(render) => {
                let style = self.resolve_render_style(context, &render.style)?;
                let mut resolved = render.clone();
                resolved.style = style;
                self.backend
                    .execute(context, &ApplicationRequest::Render(resolved))
            }
            _ => self.backend.execute(context, request),
        }
    }

    /// Executes one application request with cooperative cancellation.
    ///
    /// - `self`: Shared application service.
    /// - `context`: Trusted caller authorization and scope.
    /// - `request`: Typed application operation.
    /// - `cancelled`: Caller-owned operation cancellation flag.
    ///
    /// The canonical application result when the request completes.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns `OperationCancelled` when cancellation is observed or the
    /// backend's structured application failure.
    pub fn execute_cancellable(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ApplicationResult, ApplicationError> {
        match request {
            ApplicationRequest::Render(render) => {
                if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                    return Err(ApplicationError::OperationCancelled {
                        operation: "render".to_owned(),
                    });
                }
                let style = self.resolve_render_style(context, &render.style)?;
                let mut resolved = render.clone();
                resolved.style = style;
                self.backend.execute_cancellable(
                    context,
                    &ApplicationRequest::Render(resolved),
                    cancelled,
                )
            }
            _ => self
                .backend
                .execute_cancellable(context, request, cancelled),
        }
    }
}
