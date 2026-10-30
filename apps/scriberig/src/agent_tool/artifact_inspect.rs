//! Routes bounded provider artifact reads through the shared application tool.

use super::{
    ApplicationBackend, ApplicationContext, LocalAgentTool, Tool, ToolError, ToolResult,
    invalid_argument,
};
use scriberig_app::{CapabilityAction, CapabilityId, CapabilityScope, OperationKind};

impl<B: ApplicationBackend> LocalAgentTool<'_, B> {
    /// Reads artifact summaries under the caller's project Inspect grant.
    ///
    /// - `context`: Trusted operation context and snapshot.
    /// - `arguments`: Project identity and optional bounded page parameters.
    ///
    /// A provider-neutral bounded artifact summary page.
    ///
    /// # Returns
    /// Artifact metadata and stable handles, without source member bytes.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization denial, or application failure.
    pub(super) fn artifact_inspect(
        &self,
        context: &ApplicationContext,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let project = arguments
            .get("project")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("project"))?
            .parse::<scriberig_core::ProjectId>()
            .map_err(|error| ToolError::Application {
                category: "invalid_request".to_owned(),
                message: format!("invalid project: {error}"),
            })?;
        require_selected_project_inspect(context, project)?;
        scriberig_app::agent_tools::ArtifactInspectTool::new(self.service, project).invoke_named(
            context,
            "artifact.inspect",
            arguments,
        )
    }
}

/// Requires tool discovery and an Inspect grant bound to the selected project and snapshot.
///
/// - `context`: Trusted branch context after selected-project scoping.
/// - `project`: Untrusted project argument that must match the scoped grant.
///
/// Unit when the tool is authorized for exactly this selected project.
///
/// # Returns
/// Success when both tool and project authority are present.
///
/// # Errors
/// Returns a capability denial when either authority is missing or mismatched.
fn require_selected_project_inspect(
    context: &ApplicationContext,
    project: scriberig_core::ProjectId,
) -> Result<(), ToolError> {
    let snapshot = context
        .snapshot
        .ok_or_else(|| ToolError::CapabilityDenied {
            action: "artifact.inspect".to_owned(),
            scope: format!("project:{project}"),
        })?;
    let tool_id =
        CapabilityId::new("artifact.inspect").map_err(|error| ToolError::Application {
            category: "invalid_capability".to_owned(),
            message: error.to_owned(),
        })?;
    let scope = CapabilityScope::Custom(format!("project:{project}"));
    let inspect = context.capabilities.grants.iter().any(|grant| {
        grant.actions.contains(&CapabilityAction::Inspect)
            && grant.scope == scope
            && grant.covers(
                &CapabilityAction::Inspect,
                &scope,
                Some(snapshot),
                Some(OperationKind::Inspect),
            )
    });
    if context.capabilities.allows(&tool_id) && inspect {
        Ok(())
    } else {
        Err(ToolError::CapabilityDenied {
            action: "artifact.inspect".to_owned(),
            scope: format!("project:{project}@{snapshot}"),
        })
    }
}
