//! Routes provider-visible CAD edits through shared application tools.

use scriberig_agent_core::{Tool as _, ToolError, ToolResult};
use scriberig_app::agent_tools::{
    CAD_LAYER_CHANGE_TOOL, CAD_LINE_EDIT_TOOL, CadLayerChangeTool, CadLineEditTool,
};
use scriberig_app::{ApplicationBackend, ApplicationContext};
use scriberig_core::ProjectId;

use super::LocalAgentTool;
use crate::agent_tool_helpers::{invalid_argument, require_capability};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Creates a typed CAD line geometry candidate through the shared app tool.
    ///
    /// # Parameters
    /// - `context`: Trusted branch context containing the exact target grant.
    /// - `name`: Registered CAD operation name.
    /// - `arguments`: Project, entity, and exact start/end coordinates.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply.
    ///
    /// # Errors
    /// Returns capability, argument, cancellation, or candidate creation failures.
    pub(super) fn cad_update_line_geometry(
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
        CadLineEditTool::new(self.service.clone(), project).invoke_named_cancellable(
            context,
            CAD_LINE_EDIT_TOOL,
            arguments,
            cancelled,
        )
    }

    /// Creates a typed CAD layer-change candidate through the shared app tool.
    ///
    /// # Parameters
    /// - `context`: Trusted branch context containing the exact target grant.
    /// - `name`: Registered CAD operation name.
    /// - `arguments`: Project, entity, and destination layer identities.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply.
    ///
    /// # Errors
    /// Returns capability, argument, cancellation, or candidate creation failures.
    pub(super) fn cad_change_entity_layer(
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
        CadLayerChangeTool::new(self.service.clone(), project).invoke_named_cancellable(
            context,
            CAD_LAYER_CHANGE_TOOL,
            arguments,
            cancelled,
        )
    }
}
