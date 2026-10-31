//! Provides bounded snapshot-bound spreadsheet range reads to semantic agents.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_spreadsheet::CellRange;
use serde_json::Value;

/// Stable identifier for bounded spreadsheet cell-range reads.
pub const SPREADSHEET_RANGE_READ_TOOL: &str = "spreadsheet.read_range";

/// Reads bounded sparse cell pages through shared application authorization.
pub struct SpreadsheetRangeReadTool<B> {
    /// Shared application service enforcing Inspect scope and snapshot binding.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> SpreadsheetRangeReadTool<B> {
    /// Creates a range reader bound to one host-selected project.
    ///
    /// # Parameters
    /// - `service`: Shared application service.
    /// - `project`: Trusted project identity selected by the host.
    ///
    /// # Returns
    /// A bounded spreadsheet read tool.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for SpreadsheetRangeReadTool<B> {
    /// Returns the stable spreadsheet range-read tool name.
    ///
    /// # Returns
    /// The registered semantic tool identifier.
    fn name(&self) -> &'static str {
        SPREADSHEET_RANGE_READ_TOOL
    }

    /// Rejects generic input because range reads require named bounded arguments.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `_input`: Generic input without spreadsheet query fields.
    ///
    /// # Returns
    /// Never returns a result.
    ///
    /// # Errors
    /// Returns an invalid-input error directing callers to the named operation.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(invalid_argument(
            "spreadsheet reads require worksheet and bounded page arguments",
        ))
    }

    /// Reads one exact-snapshot sparse cell page.
    ///
    /// # Parameters
    /// - `context`: Trusted request context with exact snapshot and Inspect grant.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Worksheet, optional range, offset, and bounded limit.
    ///
    /// # Returns
    /// A bounded page with semantic cell handles and continuation metadata.
    ///
    /// # Errors
    /// Returns invalid-input, authorization, snapshot, query, or serialization failures.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &Value,
    ) -> Result<ToolResult, ToolError> {
        if name != SPREADSHEET_RANGE_READ_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown spreadsheet read tool".to_owned(),
            }));
        }
        let worksheet = arguments
            .get("worksheet")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_argument("worksheet must be an entity ID"))?
            .parse::<EntityId>()
            .map_err(|_| invalid_argument("worksheet must be a valid entity ID"))?;
        let offset = bound(arguments, "offset", 0, usize::MAX)?;
        let limit = bound(arguments, "limit", 100, 500)?;
        let range = parse_range(arguments.get("range"))?;
        let page = self
            .service
            .read_spreadsheet_cells(
                context,
                self.project,
                worksheet,
                range.as_ref(),
                offset,
                limit,
            )
            .map_err(tool_error)?;
        let summary = serde_json::to_string(&serde_json::json!({
            "project": page.project,
            "snapshot": page.snapshot,
            "worksheet": worksheet,
            "total": page.page.total,
            "offset": offset,
            "has_more": page.page.has_more,
            "cells": page.page.cells,
        }))
        .map_err(|error| ToolError::Application {
            category: "serialization_failed".to_owned(),
            message: error.to_string(),
        })?;
        Ok(ToolResult {
            entity: worksheet,
            revision: None,
            snapshot: Some(page.snapshot),
            object_type: Some("spreadsheet.cell_page".to_owned()),
            summary: Some(summary),
            handles: page
                .page
                .cells
                .iter()
                .map(|cell| cell.id.entity().to_string())
                .collect(),
        })
    }
}

/// Parses an optional inclusive rectangular cell range.
///
/// # Parameters
/// - `value`: Optional JSON range object.
///
/// # Returns
/// Validated cell bounds when supplied.
///
/// # Errors
/// Returns an invalid-input error for missing, malformed, zero, or reversed bounds.
fn parse_range(value: Option<&Value>) -> Result<Option<CellRange>, ToolError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let coordinate = |name: &str| {
        value
            .get(name)
            .and_then(Value::as_u64)
            .and_then(|number| u32::try_from(number).ok())
            .ok_or_else(|| invalid_argument(&format!("range.{name} must be a positive integer")))
    };
    let range = CellRange {
        first_row: coordinate("first_row")?,
        first_column: coordinate("first_column")?,
        last_row: coordinate("last_row")?,
        last_column: coordinate("last_column")?,
    };
    range
        .validate()
        .map_err(|_| invalid_argument("range bounds must be positive and ordered"))?;
    Ok(Some(range))
}

/// Reads one nonnegative integer argument within its configured maximum.
///
/// # Parameters
/// - `arguments`: Named tool arguments.
/// - `name`: Field to read.
/// - `default`: Value used when the field is absent.
/// - `maximum`: Largest permitted value.
///
/// # Returns
/// The validated integer value.
///
/// # Errors
/// Returns an invalid-input error for negative, malformed, or excessive values.
fn bound(
    arguments: &Value,
    name: &str,
    default: usize,
    maximum: usize,
) -> Result<usize, ToolError> {
    let Some(value) = arguments.get(name) else {
        return Ok(default);
    };
    value
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .filter(|number| *number <= maximum)
        .ok_or_else(|| invalid_argument(&format!("{name} exceeds its supported bound")))
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

/// Converts a shared application failure into a provider-neutral tool error.
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
