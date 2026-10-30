//! Provides candidate-only semantic worksheet rename operations to agents.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};
use serde_json::Value;

/// Stable identifier for semantic spreadsheet worksheet renames.
pub const SPREADSHEET_SHEET_RENAME_TOOL: &str = "spreadsheet.rename_sheet";

/// Creates worksheet rename candidates through shared application operations.
pub struct SpreadsheetSheetRenameTool<B> {
    /// Shared application service enforcing project and snapshot authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> SpreadsheetSheetRenameTool<B> {
    /// Creates a worksheet rename tool bound to one host-selected project.
    ///
    /// # Parameters
    /// - `service`: Shared application service.
    /// - `project`: Trusted project identity selected by the host.
    ///
    /// # Returns
    /// Candidate-only semantic worksheet rename tool.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for SpreadsheetSheetRenameTool<B> {
    /// Returns the stable semantic worksheet rename tool name.
    ///
    /// # Returns
    /// Registered provider-facing operation identifier.
    fn name(&self) -> &'static str {
        SPREADSHEET_SHEET_RENAME_TOOL
    }

    /// Rejects generic input because renames require named worksheet fields.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `_input`: Generic input without worksheet rename fields.
    ///
    /// # Returns
    /// Never returns a candidate.
    ///
    /// # Errors
    /// Returns an invalid-input error directing callers to the named operation.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(invalid_argument(
            "worksheet renames require worksheet and new_name arguments",
        ))
    }

    /// Creates a durable candidate renaming one exact semantic worksheet.
    ///
    /// # Parameters
    /// - `context`: Trusted context with exact snapshot and Propose grants.
    /// - `name`: Registered operation identifier.
    /// - `arguments`: Worksheet entity and requested authored name.
    ///
    /// # Returns
    /// Candidate handle suitable for shared validation and review.
    ///
    /// # Errors
    /// Returns invalid-input, authorization, stale-base, or candidate errors.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &Value,
    ) -> Result<ToolResult, ToolError> {
        if name != SPREADSHEET_SHEET_RENAME_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown spreadsheet rename operation".to_owned(),
            }));
        }
        let worksheet = arguments
            .get("worksheet")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_argument("worksheet must be an entity ID"))?
            .parse::<EntityId>()
            .map_err(|_| invalid_argument("worksheet must be a valid entity ID"))?;
        let new_name = arguments
            .get("new_name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_argument("new_name must contain non-whitespace text"))?;
        let candidate = self
            .service
            .propose_spreadsheet_sheet_rename(context, self.project, worksheet, new_name)
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity: worksheet,
            revision: None,
            snapshot: Some(candidate.base_snapshot),
            object_type: Some("spreadsheet.candidate".to_owned()),
            summary: Some(format!(
                "Candidate {} renames worksheet to {:?} in {}",
                candidate.id, new_name, candidate.path
            )),
            handles: vec![candidate.id],
        })
    }
}

/// Converts invalid arguments into a consistent provider-neutral tool error.
///
/// # Parameters
/// - `message`: Safe argument detail.
///
/// # Returns
/// Structured invalid-input error.
fn invalid_argument(message: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_tool_input".to_owned(),
        message: message.to_owned(),
    }
}

/// Converts an application failure into a provider-neutral tool error.
///
/// # Parameters
/// - `error`: Shared application failure.
///
/// # Returns
/// Structured application tool error.
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
