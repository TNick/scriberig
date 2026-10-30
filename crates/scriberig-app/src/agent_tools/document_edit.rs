//! Provides a candidate-only semantic paragraph edit tool for agents.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};

/// Stable semantic tool identifier for paragraph replacement candidates.
pub const DOCUMENT_PARAGRAPH_EDIT_TOOL: &str = "document.replace_paragraph";

/// Creates document paragraph candidates through shared application operations.
pub struct DocumentParagraphEditTool<B> {
    /// Shared application service enforcing project and snapshot authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> DocumentParagraphEditTool<B> {
    /// Creates a paragraph edit tool bound to one host-selected project.
    ///
    /// - service: Shared application service.
    /// - project: Trusted project identity selected by the host.
    ///
    /// Candidate-only semantic editing tool.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for DocumentParagraphEditTool<B> {
    /// Returns the stable document paragraph edit tool name.
    ///
    /// # Returns
    /// Registered operation name used by named invocation.
    fn name(&self) -> &'static str {
        DOCUMENT_PARAGRAPH_EDIT_TOOL
    }

    /// Rejects generic inspect-shaped input because edits require named arguments.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `_input`: Generic tool input without replacement text.
    ///
    /// # Returns
    /// Structured unsupported-operation failure.
    ///
    /// # Errors
    /// Always returns an error directing callers to the named operation contract.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::Application {
            category: "invalid_tool_input".to_owned(),
            message: "document edits require named paragraph and replacement arguments".to_owned(),
        })
    }

    /// Creates a candidate for one exact semantic paragraph target.
    ///
    /// # Parameters
    /// - `context`: Trusted application context with exact snapshot and Propose grants.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Object with `paragraph` and `replacement` string fields.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply operations.
    ///
    /// # Errors
    /// Returns an argument, authorization, stale-base, or candidate creation failure.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        if name != DOCUMENT_PARAGRAPH_EDIT_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown document tool".to_owned(),
            }));
        }
        let paragraph = arguments
            .get("paragraph")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("paragraph must be an entity ID"))?
            .parse::<EntityId>()
            .map_err(|_| invalid_argument("paragraph must be a valid entity ID"))?;
        let replacement = arguments
            .get("replacement")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("replacement must be a string"))?;
        let candidate = self
            .service
            .propose_document_paragraph_edit(context, self.project, paragraph, replacement)
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity: paragraph,
            revision: None,
            snapshot: Some(candidate.base_snapshot),
            object_type: Some("document.candidate".to_owned()),
            summary: Some(format!(
                "Candidate for {} is {:?}",
                candidate.path, candidate.validation
            )),
            handles: vec![candidate.id],
        })
    }

    /// Stops a paragraph edit before candidate creation when the branch is cancelled.
    ///
    /// # Parameters
    /// - `context`: Trusted application context.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Validated provider arguments.
    /// - `cancelled`: Branch cancellation state.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply operations.
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
                message: "document edit was cancelled before candidate creation".to_owned(),
            });
        }
        self.invoke_named(context, name, arguments)
    }
}

/// Converts argument validation failures into a consistent agent error.
///
/// # Parameters
/// - `message`: Safe argument diagnostic.
///
/// # Returns
/// Structured invalid-request tool error.
fn invalid_argument(message: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_tool_input".to_owned(),
        message: message.to_owned(),
    }
}

/// Converts application failures without exposing provider internals.
///
/// # Parameters
/// - `error`: Structured application failure.
///
/// # Returns
/// Provider-neutral tool failure.
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
