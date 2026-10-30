//! Provides a candidate-only semantic paragraph insertion tool for agents.

use crate::{ApplicationContext, ApplicationError, ApplicationService};
use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_core::{EntityId, ProjectId};

/// Stable semantic tool identifier for paragraph insertion candidates.
pub const DOCUMENT_PARAGRAPH_INSERT_TOOL: &str = "document.insert_paragraph";

/// Creates document paragraph candidates through shared application operations.
pub struct DocumentParagraphInsertTool<B> {
    /// Shared application service enforcing project and snapshot authorization.
    service: ApplicationService<B>,
    /// Project selected by the trusted host composition.
    project: ProjectId,
}

impl<B> DocumentParagraphInsertTool<B> {
    /// Creates a paragraph insertion tool bound to one host-selected project.
    ///
    /// # Parameters
    /// - `service`: Shared application service.
    /// - `project`: Trusted project identity selected by the host.
    ///
    /// # Returns
    /// Candidate-only semantic insertion tool.
    #[must_use]
    pub const fn new(service: ApplicationService<B>, project: ProjectId) -> Self {
        Self { service, project }
    }
}

impl<B> Tool<ApplicationContext> for DocumentParagraphInsertTool<B> {
    /// Returns the stable document paragraph insertion tool name.
    ///
    /// # Returns
    /// The registered semantic tool identifier.
    fn name(&self) -> &'static str {
        DOCUMENT_PARAGRAPH_INSERT_TOOL
    }

    /// Rejects generic inspect-shaped input because insertion requires named arguments.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `_input`: Generic tool input without section and paragraph text.
    ///
    /// # Returns
    /// Always an unsupported-operation error directing callers to the named operation.
    ///
    /// # Errors
    /// Always returns a structured invalid-input failure.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::Application {
            category: "invalid_tool_input".to_owned(),
            message: "document insertion requires named section and text arguments".to_owned(),
        })
    }

    /// Creates a candidate for one exact semantic section target.
    ///
    /// # Parameters
    /// - `context`: Trusted context with exact snapshot and Propose grants.
    /// - `name`: Registered semantic tool identifier.
    /// - `arguments`: Object with `section` and `text` string fields.
    ///
    /// # Returns
    /// Candidate handle suitable for shared review and apply operations.
    ///
    /// # Errors
    /// Returns argument, authorization, stale-base, or candidate creation failures.
    fn invoke_named(
        &self,
        context: &ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        if name != DOCUMENT_PARAGRAPH_INSERT_TOOL {
            return Err(tool_error(ApplicationError::InvalidRequest {
                operation: name.to_owned(),
                message: "unknown document tool".to_owned(),
            }));
        }
        let section = arguments
            .get("section")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("section must be an entity ID"))?
            .parse::<EntityId>()
            .map_err(|_| invalid_argument("section must be a valid entity ID"))?;
        let text = arguments
            .get("text")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_argument("text must be a string"))?;
        let candidate = self
            .service
            .propose_document_paragraph_insert(context, self.project, section, text)
            .map_err(tool_error)?;
        Ok(ToolResult {
            entity: section,
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

    /// Stops insertion before candidate creation when the branch is cancelled.
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
                message: "document insertion was cancelled before candidate creation".to_owned(),
            });
        }
        self.invoke_named(context, name, arguments)
    }
}

/// Maps an application failure into its provider-neutral tool error.
///
/// # Parameters
/// - `error`: Structured application failure.
///
/// # Returns
/// Tool error retaining the application category and message.
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

/// Creates a structured tool argument error.
///
/// # Parameters
/// - `message`: Safe explanation of the malformed argument.
///
/// # Returns
/// Invalid-input tool error.
fn invalid_argument(message: &str) -> ToolError {
    ToolError::Application {
        category: "invalid_tool_input".to_owned(),
        message: message.to_owned(),
    }
}
