//! Provides a candidate-only semantic spreadsheet cell edit tool.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_spreadsheet::{CellValue, Formula};
use serde_json::Value;

/// Stable semantic tool identifier for spreadsheet cell candidates.
pub const SPREADSHEET_CELL_EDIT_TOOL: &str = "spreadsheet.set_cell_value";

/// Creates typed cell candidates through shared application operations.
pub struct SpreadsheetCellEditTool<B> {
    /// Shared application service enforcing project and snapshot authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> SpreadsheetCellEditTool<B> {
    /// Creates a cell edit tool bound to one host-selected project.
    ///
    /// # Parameters
    /// - `service`: Shared application service.
    /// - `project`: Trusted project identity selected by the host.
    ///
    /// # Returns
    /// Candidate-only semantic cell edit tool.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for SpreadsheetCellEditTool<B> {
    /// Returns the stable spreadsheet cell edit tool name.
    ///
    /// # Returns
    /// The registered semantic tool identifier.
    fn name(&self) -> &'static str {
        SPREADSHEET_CELL_EDIT_TOOL
    }

    /// Rejects generic input because edits require an exact cell handle and typed value.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `_input`: Generic tool input without named spreadsheet fields.
    ///
    /// # Returns
    /// Never returns a candidate.
    ///
    /// # Errors
    /// Always returns an error directing callers to the named operation contract.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(invalid_argument(
            "spreadsheet edits require a cell entity ID and typed value arguments",
        ))
    }

    /// Creates a candidate for one exact semantic cell handle.
    ///
    /// # Parameters
    /// - `context`: Trusted context with exact snapshot and Propose grants.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Object with cell entity ID and typed value fields.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply operations.
    ///
    /// # Errors
    /// Returns an argument, authorization, stale-base, or candidate creation failure.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &Value,
    ) -> Result<ToolResult, ToolError> {
        if name != SPREADSHEET_CELL_EDIT_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown spreadsheet tool".to_owned(),
            }));
        }
        let cell = arguments
            .get("cell")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_argument("cell must be an entity ID"))?
            .parse::<EntityId>()
            .map_err(|_| invalid_argument("cell must be a valid entity ID"))?;
        let value = parse_cell_value(
            arguments
                .get("value")
                .ok_or_else(|| invalid_argument("value must be a typed cell value"))?,
            false,
        )?;
        let candidate = self
            .service
            .propose_spreadsheet_semantic_cell_edit(context, self.project, cell, &value)
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity: cell,
            revision: None,
            snapshot: Some(candidate.base_snapshot),
            object_type: Some("spreadsheet.candidate".to_owned()),
            summary: Some(format!(
                "Candidate {} for cell {} in {} is {:?}",
                candidate.id, cell, candidate.path, candidate.validation
            )),
            handles: vec![candidate.id],
        })
    }

    /// Stops a spreadsheet edit before candidate creation when the branch is cancelled.
    ///
    /// # Parameters
    /// - `context`: Trusted application context.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Validated provider arguments.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply operations.
    ///
    /// # Errors
    /// Returns cancellation or the named operation's structured failure.
    fn invoke_named_cancellable(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ToolError::Application {
                category: "operation_cancelled".to_owned(),
                message: "spreadsheet edit was cancelled before candidate creation".to_owned(),
            });
        }
        self.invoke_named(context, name, arguments)
    }
}

/// Parses a typed public wire cell value without interpreting formulas or cached results.
///
/// # Parameters
/// - `value`: JSON object tagged by its `kind` field.
/// - `cached`: Whether parsing a formula's cached scalar value.
///
/// # Returns
/// Format-independent semantic cell value.
///
/// # Errors
/// Returns an invalid-input error when the tagged value or payload is malformed.
fn parse_cell_value(value: &Value, cached: bool) -> Result<CellValue, ToolError> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_argument("value.kind must identify a supported cell value"))?;
    let payload = value.get("value");
    match kind {
        "blank" if payload.is_none() || payload == Some(&Value::Null) => Ok(CellValue::Blank),
        "text" => payload
            .and_then(Value::as_str)
            .map(|value| CellValue::Text(value.to_owned()))
            .ok_or_else(|| invalid_argument("text cell value must contain a string")),
        "number" => payload
            .and_then(Value::as_str)
            .map(|value| CellValue::Number(value.to_owned()))
            .ok_or_else(|| invalid_argument("number cell value must contain exact numeric text")),
        "boolean" => payload
            .and_then(Value::as_bool)
            .map(CellValue::Boolean)
            .ok_or_else(|| invalid_argument("boolean cell value must contain true or false")),
        "error" => payload
            .and_then(Value::as_str)
            .map(|value| CellValue::Error(value.to_owned()))
            .ok_or_else(|| invalid_argument("error cell value must contain a string")),
        "formula" if !cached => {
            let formula = payload
                .and_then(Value::as_object)
                .ok_or_else(|| invalid_argument("formula cell value must contain an object"))?;
            let expression = formula
                .get("expression")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid_argument("formula expression must be a string"))?;
            let cached_value = formula
                .get("cachedValue")
                .or_else(|| formula.get("cached_value"))
                .filter(|value| !value.is_null())
                .map(|value| parse_cell_value(value, true).map(Box::new))
                .transpose()?;
            Ok(CellValue::Formula(Formula {
                expression: expression.to_owned(),
                cached_value,
            }))
        }
        "formula" => Err(invalid_argument("cached formula values must be scalar")),
        _ => Err(invalid_argument("unsupported typed spreadsheet cell value")),
    }
}

/// Converts argument validation failures into a consistent agent error.
///
/// # Parameters
/// - `message`: Safe argument diagnostic.
///
/// # Returns
/// Structured invalid-request tool error.
fn invalid_argument(message: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_tool_input".to_owned(),
        message: message.to_owned(),
    }
}

/// Converts application failures without exposing provider internals.
///
/// # Parameters
/// - `error`: Structured application failure.
///
/// # Returns
/// Provider-neutral tool failure.
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
