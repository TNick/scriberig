//! Routes bounded spreadsheet range reads through the shared application tool.

use super::{
    ApplicationBackend, ApplicationContext, LocalAgentTool, Tool, ToolError, ToolResult,
    invalid_argument,
};
use crate::agent_tool_helpers::require_capability;
use scriberig_app::agent_tools::{SPREADSHEET_RANGE_READ_TOOL, SpreadsheetRangeReadTool};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Reads a bounded spreadsheet page under selected-workbook capability.
    ///
    /// # Parameters
    /// - `context`: Trusted branch context with the exact snapshot and grants.
    /// - `name`: Registered spreadsheet range-read capability.
    /// - `arguments`: Project, worksheet, optional range, and page bounds.
    ///
    /// # Returns
    /// Snapshot-bound sparse cells and stable semantic cell handles.
    ///
    /// # Errors
    /// Returns capability, argument, authorization, or application failures.
    pub(super) fn spreadsheet_read_range(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        require_capability(context, name)?;
        let project = arguments
            .get("project")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("project"))?
            .parse::<scriberig_core::ProjectId>()
            .map_err(|_| invalid_argument("project"))?;
        SpreadsheetRangeReadTool::new(self.service.clone(), project).invoke_named(
            context,
            SPREADSHEET_RANGE_READ_TOOL,
            arguments,
        )
    }
}
