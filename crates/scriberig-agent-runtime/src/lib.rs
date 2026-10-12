//! Implements provider-neutral agent orchestration above reusable agent tools.

use std::sync::atomic::AtomicBool;

use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_app::ApplicationContext;
use scriberig_core::EntityId;

pub mod cache;
pub mod config;
pub mod context;
pub mod events;
pub mod model_provider;
pub mod openai_compatible;
pub mod orchestration;
pub mod provider;
pub mod runtime;
pub mod stream;
pub mod tools;

pub use runtime::{AgentRuntime, cancellation_flag};

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
}

/// Compact result projected from a completed child branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildResult {
    /// Short externally meaningful outcome.
    pub summary: String,
    /// Stable semantic handles produced by the child.
    pub handles: Vec<String>,
}

/// Executes a trusted child branch on behalf of the parent runtime.
pub trait ChildExecutor {
    /// Runs one child with an explicitly bounded cancellation token.
    ///
    /// # Parameters
    /// - `context`: Parent trusted application context.
    /// - `scope`: Optional durable parent branch identity.
    /// - `request`: Validated child purpose and prompt.
    /// - `cancelled`: Shared cancellation flag for the execution tree.
    ///
    /// # Returns
    /// A compact child result suitable for the parent model context.
    ///
    /// # Errors
    /// Returns a safe child execution failure.
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
    /// Delegate focused work to a nested branch.
    Child(ChildRequest),
    /// Finish with provider-produced text.
    Final(String),
}

/// Provider boundary used by the integrated runtime.
pub trait ModelProvider {
    /// Produces the next decision from the current semantic conversation.
    ///
    /// # Parameters
    /// - `messages`: Immutable conversation and tool observations.
    ///
    /// # Returns
    /// The next tool request or final response.
    ///
    /// # Errors
    /// Returns a provider-specific failure.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String>;

    /// Produces a decision while forwarding provider-neutral text fragments.
    ///
    /// # Parameters
    /// - `messages`: Immutable conversation and tool observations.
    /// - `on_text`: Callback receiving bounded assistant-text fragments.
    ///
    /// # Returns
    /// The next tool request or final response.
    ///
    /// # Errors
    /// Returns the same provider failure as [`Self::decide`].
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
    /// # Parameters
    /// - `messages`: Conversation messages available to the provider.
    ///
    /// # Returns
    /// A final response containing the latest user prompt.
    ///
    /// # Errors
    /// Returns an error when no user prompt is available.
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
    /// # Returns
    /// The stable fallback tool name.
    fn name(&self) -> &'static str {
        "unconfigured"
    }

    /// Rejects tool execution until a concrete application tool is composed.
    ///
    /// # Parameters
    /// - `_context`: Trusted application context.
    /// - `_input`: Structured tool input to reject.
    ///
    /// # Returns
    /// Never returns a successful tool result.
    ///
    /// # Errors
    /// Always returns an unsupported-operation tool error.
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
/// # Parameters
/// - `prompt`: Prompt accepted by the one-shot product mode.
///
/// # Returns
/// Provider output and runtime messages.
///
/// # Errors
/// Returns the structured runtime failure.
pub fn run_local_one_shot(prompt: impl Into<String>) -> Result<AgentResult, AgentError> {
    run_local_agent(prompt)
}

/// Executes a deterministic local prompt through agent orchestration.
///
/// # Parameters
/// - `prompt`: Prompt accepted by the agent product mode.
///
/// # Returns
/// Provider output and lifecycle messages.
///
/// # Errors
/// Returns the structured runtime failure.
pub fn run_local_agent(prompt: impl Into<String>) -> Result<AgentResult, AgentError> {
    AgentRuntime::new(EchoProvider, UnconfiguredTool, 1).run(
        &ApplicationContext::empty(),
        prompt,
        &AtomicBool::new(false),
    )
}

/// Executes the legacy local prompt entry point.
///
/// # Parameters
/// - `prompt`: Prompt accepted by the compatibility entry point.
///
/// # Returns
/// Agent-runtime output and lifecycle messages.
///
/// # Errors
/// Returns the structured runtime failure.
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
    /// # Parameters
    /// - `self`: Agent error.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or formatter failure.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
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
}
