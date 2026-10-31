//! Routes provider-visible document edits through shared application tools.

use scriberig_agent_core::{Tool as _, ToolError, ToolResult};
use scriberig_app::agent_tools::{
    DOCUMENT_PARAGRAPH_EDIT_TOOL, DOCUMENT_TABLE_CELL_EDIT_TOOL, DocumentParagraphEditTool,
    DocumentTableCellEditTool,
};
use scriberig_app::{ApplicationBackend, ApplicationContext};
use scriberig_core::ProjectId;

use super::LocalAgentTool;
use crate::agent_tool_helpers::{invalid_argument, require_capability};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Creates a document paragraph or cell candidate through the shared app tool.
    ///
    /// # Parameters
    /// - `context`: Trusted context containing tool and project Propose grants.
    /// - `name`: Registered document operation name.
    /// - `arguments`: Exact project, semantic target, and replacement text.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply.
    ///
    /// # Errors
    /// Returns capability, argument, cancellation, or candidate creation failures.
    pub(super) fn document_edit(
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
        match name {
            DOCUMENT_PARAGRAPH_EDIT_TOOL | DOCUMENT_TABLE_CELL_EDIT_TOOL => {}
            _ => {
                return Err(ToolError::Application {
                    category: "unsupported_operation".to_owned(),
                    message: format!("unknown document edit tool: {name}"),
                });
            }
        }
        match name {
            DOCUMENT_PARAGRAPH_EDIT_TOOL => {
                DocumentParagraphEditTool::new(self.service.clone(), project)
                    .invoke_named_cancellable(context, name, arguments, cancelled)
            }
            DOCUMENT_TABLE_CELL_EDIT_TOOL => {
                DocumentTableCellEditTool::new(self.service.clone(), project)
                    .invoke_named_cancellable(context, name, arguments, cancelled)
            }
            _ => Err(ToolError::Application {
                category: "unsupported_operation".to_owned(),
                message: format!("unknown document edit tool: {name}"),
            }),
        }
    }
}
