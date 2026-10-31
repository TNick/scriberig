//! Implements the application-backed inspect tool over the shared query boundary.

use crate::{
    ApplicationContext, ApplicationError, DetailLevel, InspectRequest, QueryService,
    SnapshotResolution,
};

use scriberig_agent_core::{Tool, ToolDetail, ToolError, ToolInput, ToolResult};

use crate::capability::inspect_available;

/// Application-backed inspect tool with no model-provider assumptions.
pub struct InspectTool<S> {
    /// Shared application query service.
    service: S,
}

impl<S> InspectTool<S> {
    /// Creates an inspect tool over one application query service.
    ///
    /// - `service`: Application service implementing the shared query boundary.
    ///
    /// Provider-neutral inspect tool.
    #[must_use]
    pub const fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S: QueryService> Tool<ApplicationContext> for InspectTool<S> {
    /// Returns the stable inspect tool name.
    ///
    /// Stable inspect tool identifier.
    /// # Returns
    /// The operation result.
    fn name(&self) -> &'static str {
        "inspect.entity"
    }

    /// Delegates inspection to the shared application operation.
    ///
    /// - `context`: Trusted application context.
    /// - `input`: Entity, optional snapshot, and detail selection.
    ///
    /// Compact structured inspection result with stable handles.
    ///
    /// Returns a capability or application failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn invoke(
        &self,
        context: &ApplicationContext,
        input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        self.invoke_cancellable(context, input, &std::sync::atomic::AtomicBool::new(false))
    }
}

impl<S: QueryService> InspectTool<S> {
    /// Invokes inspection while forwarding branch cancellation to the app query.
    ///
    /// - `self`: Application-backed inspect tool.
    /// - `context`: Trusted application context.
    /// - `input`: Entity, optional snapshot, and detail selection.
    /// - `cancelled`: Shared cancellation state for the executing branch.
    ///
    /// Compact structured inspection result with stable handles.
    ///
    /// # Returns
    /// The inspect tool result.
    ///
    /// # Errors
    /// Returns capability, cancellation, or application failures.
    pub fn invoke_cancellable(
        &self,
        context: &ApplicationContext,
        input: ToolInput,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        if !inspect_available(context) {
            return Err(ToolError::CapabilityDenied {
                action: "inspect".to_owned(),
                scope: format!("entity:{}", input.entity),
            });
        }
        let request = InspectRequest {
            entity: input.entity,
            snapshot: input.snapshot.map_or_else(
                || {
                    context
                        .snapshot
                        .map_or(SnapshotResolution::Current, SnapshotResolution::At)
                },
                SnapshotResolution::At,
            ),
            detail: match input.detail {
                ToolDetail::Compact => DetailLevel::Compact,
                ToolDetail::Detailed => DetailLevel::Detailed,
            },
        };
        let result = self
            .service
            .inspect_cancellable(context, &request, cancelled)
            .map_err(map_application_error)?;
        let summary = result.description.clone().or(result.label.clone());
        Ok(ToolResult {
            entity: result.entity,
            revision: result.revision,
            snapshot: result.snapshot,
            object_type: result.object_type.map(|value| value.to_string()),
            summary,
            handles: result.capabilities,
        })
    }
}

/// Maps application errors without exposing backend implementation details.
///
/// - `error`: Structured application error.
///
/// Provider-neutral tool error.
/// # Returns
/// The operation result.
fn map_application_error(error: ApplicationError) -> ToolError {
    match error {
        ApplicationError::PermissionDenied { action, scope } => {
            ToolError::CapabilityDenied { action, scope }
        }
        other => ToolError::Application {
            category: other.category().to_owned(),
            message: other.to_string(),
        },
    }
}
