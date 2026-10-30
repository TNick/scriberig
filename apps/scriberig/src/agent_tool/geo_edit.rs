//! Routes provider-facing geospatial edits through the shared application tool.

use scriberig_agent_core::{Tool as _, ToolError, ToolResult};
use scriberig_app::agent_tools::{
    GEO_ATTRIBUTE_EDIT_TOOL, GEO_GEOMETRY_EDIT_TOOL, GeoAttributeEditTool, GeoGeometryEditTool,
};
use scriberig_app::{ApplicationBackend, ApplicationContext};
use scriberig_core::ProjectId;

use super::LocalAgentTool;
use crate::agent_tool_helpers::{invalid_argument, require_capability};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Creates an exact-snapshot geospatial attribute candidate through the shared app.
    ///
    /// # Parameters
    /// - `context`: Trusted context containing the tool and project Propose grants.
    /// - `name`: Registered geospatial operation name.
    /// - `arguments`: Project, source path, layer, feature, field, and typed value.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle for shared review and apply.
    ///
    /// # Errors
    /// Returns capability, argument, cancellation, or candidate creation failures.
    pub(super) fn geo_set_attribute(
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
        GeoAttributeEditTool::new(self.service.clone(), project).invoke_named_cancellable(
            context,
            GEO_ATTRIBUTE_EDIT_TOOL,
            arguments,
            cancelled,
        )
    }

    /// Creates an exact-snapshot geospatial geometry candidate through the shared app.
    ///
    /// # Parameters
    /// - `context`: Trusted context containing the tool and project Propose grants.
    /// - `name`: Registered geospatial operation name.
    /// - `arguments`: Project, source path, layer, feature, and typed geometry.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle for shared review and apply.
    ///
    /// # Errors
    /// Returns capability, argument, cancellation, or candidate creation failures.
    pub(super) fn geo_replace_geometry(
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
        GeoGeometryEditTool::new(self.service.clone(), project).invoke_named_cancellable(
            context,
            GEO_GEOMETRY_EDIT_TOOL,
            arguments,
            cancelled,
        )
    }
}
