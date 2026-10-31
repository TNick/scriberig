//! Routes provider-visible spreadsheet edits through the shared app tool.

use scriberig_agent_core::{Tool as _, ToolError, ToolResult};
use scriberig_app::agent_tools::{SPREADSHEET_CELL_EDIT_TOOL, SpreadsheetCellEditTool};
use scriberig_app::{ApplicationBackend, ApplicationContext};
use scriberig_core::ProjectId;

use super::LocalAgentTool;
use crate::agent_tool_helpers::{invalid_argument, require_capability};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Creates a spreadsheet candidate through the shared application operation.
    ///
    /// # Parameters
    /// - `context`: Trusted context containing tool and project Propose grants.
    /// - `name`: Registered spreadsheet operation name.
    /// - `arguments`: Exact project, worksheet, coordinate, and typed value.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply.
    ///
    /// # Errors
    /// Returns capability, argument, cancellation, or candidate creation failures.
    pub(super) fn spreadsheet_set_cell_value(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        require_capability(context, name)?;
        let project = arguments
            .get("project")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("project"))?
            .parse::<ProjectId>()
            .map_err(|_| invalid_argument("project"))?;
        let tool = SpreadsheetCellEditTool::new(self.service.clone(), project);
        tool.invoke_named_cancellable(context, SPREADSHEET_CELL_EDIT_TOOL, arguments, cancelled)
    }
}
