//! Provides a candidate-only semantic CAD line edit tool for agents.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};
use scriberig_domain_cad::Coordinate;

/// Stable semantic tool identifier for a line geometry candidate.
pub const CAD_LINE_EDIT_TOOL: &str = "cad.update_line_geometry";

/// Creates line geometry candidates through shared app operations.
pub struct CadLineEditTool<B> {
    /// Shared application service enforcing snapshot and project authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> CadLineEditTool<B> {
    /// Creates a CAD edit tool bound to one host-selected project.
    ///
    /// - service: Shared application service.
    /// - project: Trusted project identity selected by the host.
    ///
    /// Candidate-only semantic CAD tool.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for CadLineEditTool<B> {
    /// Returns the stable semantic CAD line edit tool name.
    ///
    /// # Returns
    /// The exact name registered in capability catalogs.
    fn name(&self) -> &'static str {
        CAD_LINE_EDIT_TOOL
    }

    /// Rejects generic input because line edits require named geometry values.
    ///
    /// # Parameters
    /// - `_context`: Trusted application request context.
    /// - `_input`: Generic tool input without typed line arguments.
    ///
    /// # Returns
    /// No success value because line edits require named arguments.
    ///
    /// # Errors
    /// Always returns an error directing callers to the named tool contract.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(invalid_argument(
            "CAD edits require named entity and coordinates",
        ))
    }

    /// Creates a candidate for one semantic line target.
    ///
    /// # Parameters
    /// - `context`: Trusted exact-snapshot context with a Propose grant.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Object with entity and three-dimensional start/end coordinates.
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
        if name != CAD_LINE_EDIT_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown CAD tool".to_owned(),
            }));
        }
        let entity = arguments
            .get("entity")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("entity must be an entity ID"))?
            .parse::<EntityId>()
            .map_err(|_| invalid_argument("entity must be a valid entity ID"))?;
        let start = coordinate(arguments.get("start"))?;
        let end = coordinate(arguments.get("end"))?;
        let candidate = self
            .service
            .propose_cad_line_geometry(context, self.project, entity, &start, &end)
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity,
            revision: None,
            snapshot: Some(candidate.base_snapshot),
            object_type: Some("cad.candidate".to_owned()),
            summary: Some(format!(
                "Candidate for {} is {:?}",
                candidate.path, candidate.validation
            )),
            handles: vec![candidate.id],
        })
    }

    /// Stops a CAD edit before candidate creation when the agent branch was cancelled.
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
                message: "CAD edit was cancelled before candidate creation".to_owned(),
            });
        }
        self.invoke_named(context, name, arguments)
    }
}

/// Parses one typed three-dimensional coordinate from tool arguments.
///
/// # Parameters
/// - `value`: JSON value expected to contain string x, y, and z ordinates.
///
/// # Returns
/// Exact decimal coordinate preserving authored spellings.
///
/// # Errors
/// Returns an invalid-input failure when any ordinate is missing or non-string.
fn coordinate(value: Option<&serde_json::Value>) -> Result<Coordinate, ToolError> {
    let value = value.ok_or_else(|| invalid_argument("coordinate object is required"))?;
    let ordinate = |name| {
        value
            .get(name)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("coordinate ordinates x, y, and z must be strings"))
    };
    Ok(Coordinate::new(
        ordinate("x")?,
        ordinate("y")?,
        ordinate("z")?,
    ))
}

/// Converts an invalid argument description into a tool error.
///
/// # Parameters
/// - `message`: Safe input diagnostic.
///
/// # Returns
/// Structured invalid-input error.
pub(super) fn invalid_argument(message: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_tool_input".to_owned(),
        message: message.to_owned(),
    }
}

/// Converts application failures while retaining capability denials.
///
/// # Parameters
/// - `error`: Structured application failure.
///
/// # Returns
/// Provider-neutral tool failure.
pub(super) fn tool_error(error: ApplicationError) -> ToolError {
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
