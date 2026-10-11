//! Implements provider-neutral agent orchestration above reusable agent tools.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_app::{ApplicationContext, OperationId, OperationRegistry, OperationStatus};

/// One model-facing message retained by the runtime loop.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentMessage {
    /// User-provided instruction.
    User(String),
    /// Structured tool result.
    Tool(ToolResult),
}

/// One model decision returned to the runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelDecision {
    /// Ask an application-backed tool to execute.
    Tool(ToolInput),
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
    /// - `_context`: Trusted application context for the invocation.
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

/// Executes a deterministic local prompt without claiming a model provider.
///
/// # Parameters
/// - `prompt`: Prompt accepted by the one-shot product mode.
///
/// # Returns
/// Provider output and lifecycle messages.
///
/// # Errors
/// Returns the structured runtime failure.
pub fn run_local_prompt(prompt: impl Into<String>) -> Result<AgentResult, AgentError> {
    AgentRuntime::new(EchoProvider, UnconfiguredTool, 1).run(
        &ApplicationContext::empty(),
        prompt,
        &AtomicBool::new(false),
    )
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
    /// The caller requested cancellation.
    Cancelled,
}

impl std::fmt::Display for AgentError {
    /// Formats a safe agent-runtime failure.
    ///
    /// # Parameters
    /// - `self`: Agent error to format.
    /// - `formatter`: Formatter receiving the rendered error.
    ///
    /// # Returns
    /// Formatting success or the formatter's error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(message) => write!(formatter, "agent provider: {message}"),
            Self::Tool(error) => write!(formatter, "agent tool: {error:?}"),
            Self::StepLimit { limit } => write!(formatter, "agent step limit reached: {limit}"),
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

/// Runs one model/tool loop through an explicit application context.
pub struct AgentRuntime<P, T> {
    /// Model provider producing semantic decisions.
    provider: P,
    /// Application-backed tool used by provider decisions.
    tool: T,
    /// Maximum number of provider decisions allowed.
    step_limit: usize,
    /// Optional shared registry receiving this runtime's lifecycle.
    registry: Option<OperationRegistry>,
    /// Monotonic local identity used when creating operation handles.
    next_operation: u64,
}

impl<P, T> AgentRuntime<P, T>
where
    P: ModelProvider,
    T: Tool,
{
    /// Creates a runtime with a bounded tool-loop step count.
    ///
    /// # Parameters
    /// - `provider`: Model decision provider.
    /// - `tool`: Application-backed agent tool.
    /// - `step_limit`: Maximum number of decisions before failure.
    ///
    /// # Returns
    /// A configured agent runtime.
    #[must_use]
    pub const fn new(provider: P, tool: T, step_limit: usize) -> Self {
        Self {
            provider,
            tool,
            step_limit,
            registry: None,
            next_operation: 1,
        }
    }

    /// Attaches an application operation registry to this runtime.
    ///
    /// # Parameters
    /// - `self`: Runtime being configured.
    /// - `registry`: Shared lifecycle registry.
    ///
    /// # Returns
    /// The runtime configured to publish queued, running, and terminal states.
    #[must_use]
    pub fn with_operation_registry(mut self, registry: OperationRegistry) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Executes one prompt until a final model response or failure.
    ///
    /// # Parameters
    /// - `self`: Runtime to execute.
    /// - `context`: Trusted application request context.
    /// - `prompt`: User instruction.
    /// - `cancelled`: Shared cancellation flag.
    ///
    /// # Returns
    /// Final model output and tool observations.
    ///
    /// # Errors
    /// Returns provider, tool, step-limit, or cancellation failures.
    pub fn run(
        &mut self,
        context: &ApplicationContext,
        prompt: impl Into<String>,
        cancelled: &AtomicBool,
    ) -> Result<AgentResult, AgentError> {
        let operation = self.start_operation()?;
        let mut messages = vec![AgentMessage::User(prompt.into())];
        for _ in 0..self.step_limit {
            if cancelled.load(Ordering::Acquire) {
                self.finish_operation(operation.as_ref(), OperationStatus::Cancelled);
                return Err(AgentError::Cancelled);
            }
            match self.provider.decide(&messages).map_err(|error| {
                self.finish_operation(operation.as_ref(), OperationStatus::Failed);
                AgentError::Provider(error)
            })? {
                ModelDecision::Final(output) => {
                    self.finish_operation(operation.as_ref(), OperationStatus::Completed);
                    return Ok(AgentResult { output, messages });
                }
                ModelDecision::Tool(input) => {
                    let result = self.tool.invoke(context, input).map_err(|error| {
                        self.finish_operation(operation.as_ref(), OperationStatus::Failed);
                        AgentError::Tool(error)
                    })?;
                    messages.push(AgentMessage::Tool(result));
                }
            }
        }
        self.finish_operation(operation.as_ref(), OperationStatus::Failed);
        Err(AgentError::StepLimit {
            limit: self.step_limit,
        })
    }

    /// Starts one tracked agent operation when a registry is configured.
    ///
    /// # Returns
    /// The operation identity when lifecycle tracking is enabled.
    ///
    /// # Errors
    /// Returns a provider error when an operation cannot be registered.
    fn start_operation(&mut self) -> Result<Option<OperationId>, AgentError> {
        let Some(registry) = &self.registry else {
            return Ok(None);
        };
        let id = OperationId::new(format!("agent-{}", self.next_operation))
            .map_err(|error| AgentError::Provider(error.to_string()))?;
        self.next_operation += 1;
        registry
            .start(id.clone(), scriberig_app::OperationKind::Query)
            .map_err(|error| AgentError::Provider(error.to_string()))?;
        registry
            .transition(&id, OperationStatus::Running)
            .map_err(|error| AgentError::Provider(error.to_string()))?;
        Ok(Some(id))
    }

    /// Publishes a terminal state for one tracked agent operation.
    ///
    /// # Parameters
    /// - `operation`: Optional operation identity to update.
    /// - `status`: Terminal lifecycle status to publish.
    fn finish_operation(&self, operation: Option<&OperationId>, status: OperationStatus) {
        if let (Some(registry), Some(operation)) = (&self.registry, operation) {
            let _ = registry.transition(operation, status);
        }
    }
}

/// Creates a cancellation flag suitable for one runtime invocation.
///
/// # Returns
/// A clear, shared cancellation flag.
#[must_use]
pub fn cancellation_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
