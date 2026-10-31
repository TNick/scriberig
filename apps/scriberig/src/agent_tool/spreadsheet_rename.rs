//! Routes provider-visible worksheet renames through shared app candidate operations.

use super::{
    ApplicationBackend, ApplicationContext, LocalAgentTool, Tool, ToolError, ToolResult,
    invalid_argument,
};
use crate::agent_tool_helpers::require_capability;
use scriberig_app::agent_tools::{SPREADSHEET_SHEET_RENAME_TOOL, SpreadsheetSheetRenameTool};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Proposes a worksheet rename under exact-snapshot project authority.
    ///
    /// # Parameters
    /// - `context`: Trusted branch context with the exact snapshot and tool grant.
    /// - `name`: Registered spreadsheet rename capability.
    /// - `arguments`: Project, worksheet, and requested sheet name.
    ///
    /// # Returns
    /// Candidate handle suitable for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns capability, argument, snapshot, or candidate creation failures.
    pub(super) fn spreadsheet_rename_sheet(
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
        SpreadsheetSheetRenameTool::new(self.service.clone(), project).invoke_named(
            context,
            SPREADSHEET_SHEET_RENAME_TOOL,
            arguments,
        )
    }
}
