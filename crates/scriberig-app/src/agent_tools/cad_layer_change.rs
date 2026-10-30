//! Provides a candidate-only semantic CAD layer-change tool for agents.

use crate::agent_tools::cad_edit::{invalid_argument, tool_error};
use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};

/// Stable semantic tool identifier for changing an entity's layer.
pub const CAD_LAYER_CHANGE_TOOL: &str = "cad.change_entity_layer";

/// Creates layer-change candidates through shared app operations.
///
/// # Public members
/// - `new`: Binds the tool to a host-selected project.
/// - `name`: Returns the stable tool identity.
/// - `invoke`: Rejects unstructured input.
/// - `invoke_named`: Creates an exact-snapshot semantic candidate.
/// - `invoke_named_cancellable`: Stops before candidate creation when cancelled.
///
/// # Private members
/// - `service`: Shared application service enforcing authorization.
/// - `project`: Project selected by trusted host composition.
pub struct CadLayerChangeTool<B> {
    /// Shared application service enforcing snapshot and project authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> CadLayerChangeTool<B> {
    /// Creates a CAD layer tool bound to one host-selected project.
    ///
    /// # Parameters
    /// - `service`: Shared application service.
    /// - `project`: Trusted project identity selected by the host.
    ///
    /// # Returns
    /// Candidate-only semantic CAD tool.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for CadLayerChangeTool<B> {
    /// Returns the stable semantic CAD layer-change tool name.
    ///
    /// # Returns
    /// The exact name registered in capability catalogs.
    fn name(&self) -> &'static str {
        CAD_LAYER_CHANGE_TOOL
    }

    /// Rejects generic input because layer changes require named semantic handles.
    ///
    /// # Parameters
    /// - `_context`: Trusted application request context.
    /// - `_input`: Generic tool input without typed entity and layer arguments.
    ///
    /// # Returns
    /// No value because this tool requires its named argument contract.
    ///
    /// # Errors
    /// Always returns an error directing callers to the named tool contract.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(invalid_argument(
            "CAD layer changes require named entity and destination_layer IDs",
        ))
    }

    /// Creates a candidate for one semantic entity and destination layer.
    ///
    /// # Parameters
    /// - `context`: Trusted exact-snapshot context with a Propose grant.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Object with entity and `destination_layer` identities.
    ///
    /// # Returns
    /// Candidate handle for shared review and validation.
    ///
    /// # Errors
    /// Returns invalid arguments, authorization, stale-base, or candidate errors.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        if name != CAD_LAYER_CHANGE_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown CAD tool".to_owned(),
            }));
        }
        let entity = parse_identity(arguments, "entity")?;
        let destination_layer = parse_identity(arguments, "destination_layer")?;
        let candidate = self
            .service
            .propose_cad_layer_change(context, self.project, entity, destination_layer)
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity,
            revision: None,
            snapshot: Some(candidate.base_snapshot),
            object_type: Some("cad.candidate".to_owned()),
            summary: Some(format!(
                "Layer change candidate for {} is {:?}",
                candidate.path, candidate.validation
            )),
            handles: vec![candidate.id],
        })
    }

    /// Stops a layer change before candidate creation when the agent branch was cancelled.
    ///
    /// # Parameters
    /// - `context`: Trusted application request context.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Provider-supplied typed argument object.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review.
    ///
    /// # Errors
    /// Returns cancellation or the named operation's structured failure.
    fn invoke_named_cancellable(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ToolError::Application {
                category: "operation_cancelled".to_owned(),
                message: "CAD layer change was cancelled before candidate creation".to_owned(),
            });
        }
        self.invoke_named(context, name, arguments)
    }
}

/// Parses one exact semantic identity from named tool arguments.
///
/// # Parameters
/// - `arguments`: Provider-supplied JSON object.
/// - `key`: Required field name.
///
/// # Returns
/// Parsed semantic entity identity.
///
/// # Errors
/// Returns an invalid-input failure when the field is absent, non-string, or malformed.
fn parse_identity(arguments: &serde_json::Value, key: &str) -> Result<EntityId, ToolError> {
    arguments
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_argument(&format!("{key} must be an entity ID")))?
        .parse::<EntityId>()
        .map_err(|_| invalid_argument(&format!("{key} must be a valid entity ID")))
}
