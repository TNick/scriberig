//! Routes provider-visible QGIS renames through shared application candidates.

use super::{
    ApplicationBackend, ApplicationContext, LocalAgentTool, Tool, ToolError, ToolResult,
    invalid_argument, require_capability,
};
use scriberig_app::agent_tools::{QGIS_LAYER_RENAME_TOOL, QgisLayerRenameTool};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Proposes a QGIS layer-reference rename under exact-snapshot authority.
    ///
    /// # Parameters
    /// - `context`: Trusted branch context with exact snapshot and tool grant.
    /// - `name`: Registered QGIS rename capability.
    /// - `arguments`: Project, reference identity, and requested display name.
    ///
    /// # Returns
    /// Candidate handle suitable for shared validation, review, and apply.
    ///
    /// # Errors
    /// Returns capability, argument, snapshot, or candidate creation failures.
    pub(super) fn qgis_rename_layer(
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
        QgisLayerRenameTool::new(self.service.clone(), project).invoke_named(
            context,
            QGIS_LAYER_RENAME_TOOL,
            arguments,
        )
    }
}
