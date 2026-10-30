//! Provides a candidate-only semantic table-cell edit tool for agents.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};

/// Stable semantic tool identifier for table-cell replacement candidates.
pub const DOCUMENT_TABLE_CELL_EDIT_TOOL: &str = "document.update_table_cell";

/// Creates DOCX table-cell candidates through shared application operations.
pub struct DocumentTableCellEditTool<B> {
    /// Shared application service enforcing project and snapshot authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> DocumentTableCellEditTool<B> {
    /// Creates a table-cell edit tool bound to one host-selected project.
    ///
    /// # Parameters
    /// - `service`: Shared application service.
    /// - `project`: Trusted project identity selected by the host.
    ///
    /// # Returns
    /// Candidate-only semantic table-cell editing tool.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for DocumentTableCellEditTool<B> {
    /// Returns the stable table-cell edit tool name.
    ///
    /// # Returns
    /// Registered operation name used by named invocation.
    fn name(&self) -> &'static str {
        DOCUMENT_TABLE_CELL_EDIT_TOOL
    }

    /// Rejects generic input because table-cell edits require named arguments.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `_input`: Generic tool input without a target cell and replacement.
    ///
    /// # Returns
    /// Always returns an invalid-tool-input error directing callers to named arguments.
    ///
    /// # Errors
    /// Always returns an error because unnamed generic input cannot identify a cell edit.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::Application {
            category: "invalid_tool_input".to_owned(),
            message: "table-cell edits require named cell and replacement arguments".to_owned(),
        })
    }

    /// Creates a candidate for one exact semantic table-cell target.
    ///
    /// # Parameters
    /// - `context`: Trusted context with exact snapshot and Propose grants.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Object with `cell` and `replacement` string fields.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply operations.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization, stale-base, or candidate failures.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        if name != DOCUMENT_TABLE_CELL_EDIT_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown document tool".to_owned(),
            }));
        }
        let cell = arguments
            .get("cell")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("cell must be an entity ID"))?
            .parse::<EntityId>()
            .map_err(|_| invalid_argument("cell must be a valid entity ID"))?;
        let replacement = arguments
            .get("replacement")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("replacement must be a string"))?;
        let candidate = self
            .service
            .propose_document_table_cell_edit(context, self.project, cell, replacement)
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity: cell,
            revision: None,
            snapshot: Some(candidate.base_snapshot),
            object_type: Some("document.candidate".to_owned()),
            summary: Some(format!(
                "Table-cell candidate for {} is {:?}",
                candidate.path, candidate.validation
            )),
            handles: vec![candidate.id],
        })
    }

    /// Stops a cell edit before candidate creation when the branch is cancelled.
    ///
    /// # Parameters
    /// - `context`: Trusted application context.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Validated provider arguments.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle or operation-cancelled error.
    ///
    /// # Errors
    /// Returns cancellation or the named operation's structured failure.
    fn invoke_named_cancellable(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ToolError::Application {
                category: "operation_cancelled".to_owned(),
                message: "table-cell edit was cancelled before candidate creation".to_owned(),
            });
        }
        self.invoke_named(context, name, arguments)
    }
}

/// Creates a safe table-cell tool argument error.
///
/// # Parameters
/// - `message`: Safe validation detail.
///
/// # Returns
/// Structured invalid-argument tool error.
fn invalid_argument(message: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_tool_input".to_owned(),
        message: message.to_owned(),
    }
}

/// Converts an application failure into a provider-safe tool error.
///
/// # Parameters
/// - `error`: Shared application failure.
///
/// # Returns
/// Structured tool error with the operation code and safe message.
fn tool_error(error: ApplicationError) -> ToolError {
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
