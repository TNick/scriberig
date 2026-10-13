//! Defines the provider-neutral agent tool contract.

use scriberig_core::{EntityId, RevisionId, SnapshotId};

/// Selects compact or detailed semantic inspection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolDetail {
    /// Return orientation metadata and stable handles.
    Compact,
    /// Return additional summaries exposed by the application.
    Detailed,
}

/// Provider-neutral input for an application-backed tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolInput {
    /// Entity to inspect.
    pub entity: EntityId,
    /// Optional exact snapshot context.
    pub snapshot: Option<SnapshotId>,
    /// Requested result detail.
    pub detail: ToolDetail,
}

/// Compact semantic result suitable for progressive disclosure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolResult {
    /// Inspected entity identity.
    pub entity: EntityId,
    /// Exact resolved revision, when available.
    pub revision: Option<RevisionId>,
    /// Exact snapshot used by the operation, when available.
    pub snapshot: Option<SnapshotId>,
    /// Semantic object type, when available.
    pub object_type: Option<String>,
    /// Short label or description, when available.
    pub summary: Option<String>,
    /// Stable handles for follow-up semantic actions.
    pub handles: Vec<String>,
}

/// Structured provider-neutral tool failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolError {
    /// The application denied the requested capability.
    CapabilityDenied {
        /// Denied action.
        action: String,
        /// Denied scope.
        scope: String,
    },
    /// The application returned another structured failure category.
    Application {
        /// Stable application error category.
        category: String,
        /// Safe diagnostic message.
        message: String,
    },
}

/// Describes and invokes one provider-neutral application tool.
pub trait Tool {
    /// Returns the stable tool identifier.
    ///
    /// # Returns
    /// Stable provider-neutral tool name.
    fn name(&self) -> &'static str;

    /// Invokes the tool with semantic input and trusted request context.
    ///
    /// # Parameters
    /// - `self`: Tool implementation.
    /// - `context`: Trusted application context.
    /// - `input`: Provider-neutral semantic input.
    ///
    /// # Returns
    /// Compact structured tool result.
    ///
    /// # Errors
    /// Returns a structured capability or application failure.
    fn invoke(
        &self,
        context: &scriberig_app::ApplicationContext,
        input: ToolInput,
    ) -> Result<ToolResult, ToolError>;

    /// Invokes a named provider-facing control tool.
    ///
    /// # Parameters
    /// - `self`: Tool implementation.
    /// - `_context`: Trusted application context.
    /// - `name`: Registered control-tool identifier.
    /// - `_arguments`: Validated provider arguments.
    ///
    /// # Returns
    /// Compact structured control output.
    ///
    /// # Errors
    /// Returns unsupported-operation unless the composed tool owns the name.
    fn invoke_named(
        &self,
        _context: &scriberig_app::ApplicationContext,
        name: &str,
        _arguments: &serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::Application {
            category: "unsupported_operation".to_owned(),
            message: format!("agent tool is not configured: {name}"),
        })
    }

    /// Invokes a named provider-facing control tool with branch cancellation.
    ///
    /// # Parameters
    /// - `self`: Tool implementation.
    /// - `context`: Trusted application context.
    /// - `name`: Registered control-tool identifier.
    /// - `arguments`: Validated provider arguments.
    /// - `cancelled`: Shared cancellation flag for the current branch.
    ///
    /// # Returns
    /// Compact structured control output.
    ///
    /// # Errors
    /// Returns unsupported-operation unless the composed tool owns the name.
    fn invoke_named_cancellable(
        &self,
        context: &scriberig_app::ApplicationContext,
        name: &str,
        arguments: &serde_json::Value,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<ToolResult, ToolError> {
        let _ = cancelled;
        self.invoke_named(context, name, arguments)
    }
}
