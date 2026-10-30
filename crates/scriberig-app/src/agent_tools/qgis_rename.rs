//! Provides candidate-only QGIS layer-reference rename operations to agents.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};
use serde_json::Value;

/// Stable identifier for semantic QGIS layer-reference renames.
pub const QGIS_LAYER_RENAME_TOOL: &str = "qgis.rename_layer_reference";

/// Creates QGIS rename candidates through shared application operations.
pub struct QgisLayerRenameTool<B> {
    /// Shared application service enforcing project and snapshot authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> QgisLayerRenameTool<B> {
    /// Creates a QGIS rename tool bound to one host-selected project.
    ///
    /// # Parameters
    /// - `service`: Shared application service.
    /// - `project`: Trusted project identity selected by the host.
    ///
    /// # Returns
    /// Candidate-only semantic QGIS rename tool.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for QgisLayerRenameTool<B> {
    /// Returns the stable semantic QGIS rename tool name.
    ///
    /// # Returns
    /// Registered provider-facing operation identifier.
    fn name(&self) -> &'static str {
        QGIS_LAYER_RENAME_TOOL
    }

    /// Rejects generic input because renames require named reference fields.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `_input`: Generic input without QGIS rename fields.
    ///
    /// # Returns
    /// Never returns a candidate.
    ///
    /// # Errors
    /// Returns an invalid-input error directing callers to the named operation.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(invalid_argument(
            "QGIS rename requires reference and new_name arguments",
        ))
    }

    /// Creates a durable candidate renaming one exact semantic layer reference.
    ///
    /// # Parameters
    /// - `context`: Trusted context with exact snapshot and Propose grants.
    /// - `name`: Registered operation identifier.
    /// - `arguments`: QGIS reference entity and requested authored name.
    ///
    /// # Returns
    /// Candidate handle suitable for shared validation and review.
    ///
    /// # Errors
    /// Returns invalid-input, authorization, stale-base, or candidate errors.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &Value,
    ) -> Result<ToolResult, ToolError> {
        if name != QGIS_LAYER_RENAME_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown QGIS rename operation".to_owned(),
            }));
        }
        let reference = arguments
            .get("reference")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid_argument("reference must be an entity ID"))?
            .parse::<EntityId>()
            .map_err(|_| invalid_argument("reference must be a valid entity ID"))?;
        let new_name = arguments
            .get("new_name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid_argument("new_name must contain non-whitespace text"))?;
        let candidate = self
            .service
            .propose_qgis_layer_rename(context, self.project, reference, new_name)
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity: reference,
            revision: None,
            snapshot: Some(candidate.base_snapshot),
            object_type: Some("qgis.candidate".to_owned()),
            summary: Some(format!(
                "Candidate {} renames QGIS layer reference to {:?} in {}",
                candidate.id, new_name, candidate.path
            )),
            handles: vec![candidate.id],
        })
    }
}

/// Converts invalid arguments into a provider-neutral tool error.
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

/// Converts an application failure into a provider-neutral tool error.
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
