//! Implements provider-neutral agent orchestration above reusable agent tools.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use scriberig_agent_core::{Tool, ToolError, ToolInput, ToolResult};
use scriberig_app::ApplicationContext;

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
    fn name(&self) -> &'static str {
        "unconfigured"
    }

    /// Rejects tool execution until a concrete application tool is composed.
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
        }
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
        let mut messages = vec![AgentMessage::User(prompt.into())];
        for _ in 0..self.step_limit {
            if cancelled.load(Ordering::Acquire) {
                return Err(AgentError::Cancelled);
            }
            match self
                .provider
                .decide(&messages)
                .map_err(AgentError::Provider)?
            {
                ModelDecision::Final(output) => return Ok(AgentResult { output, messages }),
                ModelDecision::Tool(input) => {
                    let result = self.tool.invoke(context, input).map_err(AgentError::Tool)?;
                    messages.push(AgentMessage::Tool(result));
                }
            }
        }
        Err(AgentError::StepLimit {
            limit: self.step_limit,
        })
    }
}

/// Creates a cancellation flag suitable for one runtime invocation.
#[must_use]
pub fn cancellation_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

#[cfg(test)]
mod tests {
    use super::{AgentMessage, AgentResult, AgentRuntime, ModelDecision, ModelProvider};
    use scriberig_agent_core::{InspectTool, ToolDetail, ToolInput};
    use scriberig_app::{
        ApplicationContext, CapabilityContext, CapabilityId, InspectRequest, InspectResult,
        QueryService,
    };
    use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};
    use std::sync::atomic::AtomicBool;

    /// Provides one deterministic query service to the runtime test.
    struct Fixture;

    impl QueryService for Fixture {
        /// Returns one deterministic inspection.
        fn inspect(
            &self,
            _context: &ApplicationContext,
            request: &InspectRequest,
        ) -> Result<InspectResult, scriberig_app::ApplicationError> {
            Ok(InspectResult {
                entity: request.entity,
                revision: Some(RevisionId::new([2; 32])),
                snapshot: Some(SnapshotId::new([3; 32])),
                object_type: Some(ObjectType::new("test.entity").expect("type")),
                label: Some("fixture".to_owned()),
                description: None,
                capabilities: vec!["inspect.entity".to_owned()],
                trust: None,
            })
        }
    }

    /// Provides one tool request followed by a final answer.
    struct Provider {
        calls: usize,
    }

    impl ModelProvider for Provider {
        /// Produces a deterministic inspect call then final text.
        fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
            if self.calls == 0 {
                self.calls += 1;
                return Ok(ModelDecision::Tool(ToolInput {
                    entity: EntityId::new([1; 16]),
                    snapshot: None,
                    detail: ToolDetail::Compact,
                }));
            }
            assert!(matches!(messages.last(), Some(AgentMessage::Tool(_))));
            Ok(ModelDecision::Final("done".to_owned()))
        }
    }

    /// Verifies the runtime executes tools only through agent-core contracts.
    #[test]
    fn runtime_completes_provider_tool_loop() {
        let mut runtime = AgentRuntime::new(Provider { calls: 0 }, InspectTool::new(Fixture), 2);
        let context = ApplicationContext {
            capabilities: CapabilityContext::new([
                CapabilityId::new("inspect.entity").expect("capability")
            ]),
            ..ApplicationContext::empty()
        };
        let result = runtime
            .run(&context, "inspect", &AtomicBool::new(false))
            .expect("agent result");
        assert_eq!(result.output, "done");
        assert_eq!(result.messages.len(), 2);
        let _: AgentResult = result;
    }
}
