//! Implements provider-neutral agent orchestration above reusable agent tools.

use std::sync::atomic::AtomicBool;

use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_app::ApplicationContext;
use scriberig_core::EntityId;

pub mod cache;
pub mod config;
pub mod context;
pub mod coordinator;
mod credential_redaction;
pub mod events;
pub mod model_provider;
pub mod openai_compatible;
pub mod orchestration;
pub mod plan_context;
pub mod profile;
pub mod provider;
pub mod runtime;
pub mod scheduler;
pub mod stream;
pub mod tools;
pub mod upstream_tool;

pub use runtime::{AgentRuntime, cancellation_flag};

/// Thread-safe sink for provider-neutral assistant text fragments.
pub type OutputCallback = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

/// One model-facing message retained by the runtime loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentMessage {
    /// User-provided instruction.
    User(String),
    /// Structured tool result.
    Tool(ToolResult),
    /// Compact result returned by a delegated child branch.
    Child(ChildResult),
}

/// Trusted request to start a nested reasoning branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildRequest {
    /// Purpose assigned to the child branch.
    pub purpose: String,
    /// Bounded prompt inherited by the child.
    pub prompt: String,
    /// Capability identifiers explicitly delegated to the child.
    pub capabilities: Vec<String>,
    /// Compact result shape expected from the child.
    pub expected_result: String,
}

/// Identifies the durable parent branch for delegated work.
#[derive(Clone, Debug)]
pub struct ChildExecutionScope {
    /// Session containing the parent branch.
    pub session: EntityId,
    /// Parent branch receiving the child result reference.
    pub parent_branch: EntityId,
    /// Zero-based nested branch depth of the parent.
    pub depth: usize,
    /// Shared active-branch counter for the execution tree.
    pub active_branches: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// Exact semantic project scope inherited by this branch tree.
    pub semantic_scope: Option<context::ProjectSemanticScope>,
}

/// Compact result projected from a completed child branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildResult {
    /// Short externally meaningful outcome.
    pub summary: String,
    /// Stable semantic handles produced by the child.
    pub handles: Vec<String>,
    /// Durable branch identity when the child executor persists a session branch.
    pub branch: Option<EntityId>,
}

/// Executes a trusted child branch on behalf of the parent runtime.
pub trait ChildExecutor {
    /// Runs one child with an explicitly bounded cancellation token.
    ///
    /// - `context`: Parent trusted application context.
    /// - `scope`: Optional durable parent branch identity.
    /// - `request`: Validated child purpose and prompt.
    /// - `cancelled`: Shared cancellation flag for the execution tree.
    ///
    /// A compact child result suitable for the parent model context.
    ///
    /// Returns a safe child execution failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn execute_child(
        &mut self,
        context: &ApplicationContext,
        scope: Option<ChildExecutionScope>,
        request: ChildRequest,
        cancelled: &AtomicBool,
    ) -> Result<ChildResult, String>;
}

/// One model decision returned to the runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelDecision {
    /// Ask an application-backed tool to execute.
    Tool(ToolInput),
    /// Ask a named application control tool to execute.
    NamedTool {
        /// Registered provider-facing tool name.
        name: String,
        /// Validated JSON arguments supplied by the provider.
        arguments: serde_json::Value,
        /// Schema-redacted arguments safe for detailed presentation.
        presentation_arguments: serde_json::Value,
        /// Optional trusted compact label for human presentation.
        presentation_label: Option<String>,
        /// Trusted semantic category of the invoked application capability.
        category: scriberig_session::ToolActivityKind,
    },
    /// Delegate focused work to a nested branch.
    Child(ChildRequest),
    /// Finish with provider-produced text.
    Final(String),
}

/// Provider boundary used by the integrated runtime.
pub trait ModelProvider {
    /// Produces the next decision from the current semantic conversation.
    ///
    /// - `messages`: Immutable conversation and tool observations.
    ///
    /// The next tool request or final response.
    ///
    /// Returns a provider-specific failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String>;

    /// Returns and clears captured safe request projections since the last call.
    ///
    /// Provider-neutral requests prepared immediately before provider calls.
    /// # Returns
    /// The operation result.
    fn take_request_projections(&mut self) -> Vec<scriberig_session::ModelRequestProjection> {
        Vec::new()
    }

    /// Produces a decision while forwarding provider-neutral text fragments.
    ///
    /// - `messages`: Immutable conversation and tool observations.
    /// - `on_text`: Callback receiving bounded assistant-text fragments.
    ///
    /// The next tool request or final response.
    ///
    /// Returns the same provider failure as [`Self::decide`].
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn decide_with_output(
        &mut self,
        messages: &[AgentMessage],
        on_text: &mut dyn FnMut(&str),
    ) -> Result<ModelDecision, String> {
        let decision = self.decide(messages)?;
        if let ModelDecision::Final(output) = &decision {
            on_text(output);
        }
        Ok(decision)
    }
}

/// Deterministic provider used by local one-shot composition and tests.
pub struct EchoProvider;

impl ModelProvider for EchoProvider {
    /// Returns the latest user prompt as a completed response.
    ///
    /// - `messages`: Conversation messages available to the provider.
    ///
    /// A final response containing the latest user prompt.
    ///
    /// Returns an error when no user prompt is available.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        let Some(AgentMessage::User(prompt)) = messages.first() else {
            return Err("agent prompt is missing".to_owned());
        };
        Ok(ModelDecision::Final(prompt.clone()))
    }
}

/// Tool implementation that prevents unconfigured providers from mutating state.
pub struct UnconfiguredTool;

impl Tool for UnconfiguredTool {
    /// Identifies the guarded fallback tool.
    ///
    /// The stable fallback tool name.
    /// # Returns
    /// The operation result.
    fn name(&self) -> &'static str {
        "unconfigured"
    }

    /// Rejects tool execution until a concrete application tool is composed.
    ///
    /// - `_context`: Trusted application context.
    /// - `_input`: Structured tool input to reject.
    ///
    /// Never returns a successful tool result.
    ///
    /// Always returns an unsupported-operation tool error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn invoke(
        &self,
        _context: &ApplicationContext,
        _input: ToolInput,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::Application {
            category: "unsupported_operation".to_owned(),
            message: "no agent application tool is configured".to_owned(),
        })
    }
}

/// Executes a deterministic one-shot prompt through the shared runtime.
///
/// - `prompt`: Prompt accepted by the one-shot product mode.
///
/// Provider output and runtime messages.
///
/// Returns the structured runtime failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn run_local_one_shot(prompt: impl Into<String>) -> Result<AgentResult, AgentError> {
    run_local_agent(prompt)
}

/// Executes a deterministic local prompt through agent orchestration.
///
/// - `prompt`: Prompt accepted by the agent product mode.
///
/// Provider output and lifecycle messages.
///
/// Returns the structured runtime failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn run_local_agent(prompt: impl Into<String>) -> Result<AgentResult, AgentError> {
    AgentRuntime::new(EchoProvider, UnconfiguredTool, 1).run(
        &ApplicationContext::empty(),
        prompt,
        &AtomicBool::new(false),
    )
}

/// Executes the legacy local prompt entry point.
///
/// - `prompt`: Prompt accepted by the compatibility entry point.
///
/// Agent-runtime output and lifecycle messages.
///
/// Returns the structured runtime failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn run_local_prompt(prompt: impl Into<String>) -> Result<AgentResult, AgentError> {
    run_local_agent(prompt)
}

/// Structured failure returned by the agent orchestration boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentError {
    /// The model provider failed to produce a decision.
    Provider(String),
    /// A tool returned a structured semantic failure.
    Tool(ToolError),
    /// The configured step limit was reached.
    StepLimit {
        /// Configured maximum number of provider decisions.
        limit: usize,
    },
    /// A non-step runtime budget was exhausted.
    BudgetExceeded {
        /// Stable budget category.
        kind: &'static str,
        /// Configured maximum that was reached.
        limit: usize,
    },
    /// The caller requested cancellation.
    Cancelled,
}

impl std::fmt::Display for AgentError {
    /// Formats a safe agent-runtime failure.
    ///
    /// - `self`: Agent error.
    /// - `formatter`: Destination formatter.
    ///
    /// Formatting success or formatter failure.
    ///
    /// Returns the formatter error when output cannot be written.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(message) => write!(formatter, "agent provider: {message}"),
            Self::Tool(error) => write!(formatter, "agent tool: {error:?}"),
            Self::StepLimit { limit } => write!(formatter, "agent step limit reached: {limit}"),
            Self::BudgetExceeded { kind, limit } => {
                write!(formatter, "agent {kind} budget exceeded: {limit}")
            }
            Self::Cancelled => formatter.write_str("agent operation cancelled"),
        }
    }
}

impl std::error::Error for AgentError {}

/// Final result of one provider/tool execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentResult {
    /// Provider-produced final text.
    pub output: String,
    /// Tool observations collected before completion.
    pub messages: Vec<AgentMessage>,
    /// Executed tool calls with exact arguments and bounded structured results.
    pub tool_evidence: Vec<scriberig_app::ToolExecutionEvidence>,
    /// Bounded safe requests captured immediately before provider calls.
    pub model_requests: Vec<scriberig_session::ModelRequestProjection>,
}
