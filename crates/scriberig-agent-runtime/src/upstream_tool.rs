//! Adapts Scriberig-approved tool schemas and executors to `rpi-agent`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use rpi_agent::{AgentError as UpstreamAgentError, AgentTool, ToolExecutionMode};
use rpi_ai::types::{Schema, Tool};
use tokio_util::sync::CancellationToken;

use crate::provider::ToolDefinition;

/// Future returned by a Scriberig semantic tool executor.
pub type SemanticToolFuture =
    Pin<Box<dyn Future<Output = Result<rpi_agent::AgentToolResult, UpstreamAgentError>> + Send>>;

/// Executes one pre-authorized Scriberig semantic tool through `rpi-agent`.
///
/// Tool exposure and capability filtering remain Scriberig-owned. Construct
/// instances only from the branch-visible tool catalog.
///
/// # Public members
/// None.
///
/// # Private members
/// - `schema`: Upstream provider-facing tool schema.
/// - `label`: Safe label used by upstream tool events and presentation.
/// - `execute`: Scriberig semantic operation executor.
pub struct SemanticAgentTool {
    schema: Tool,
    label: String,
    execute: Arc<SemanticToolExecutor>,
}

/// Callback that runs a validated semantic invocation.
type SemanticToolExecutor = dyn Fn(
        String,
        serde_json::Value,
        CancellationToken,
        Arc<dyn Fn(rpi_agent::ToolResultPartial) + Send + Sync>,
    ) -> SemanticToolFuture
    + Send
    + Sync;

impl SemanticAgentTool {
    /// Creates an upstream tool adapter from a Scriberig-approved definition.
    ///
    /// The caller must select the definition from the current branch's
    /// capability-filtered catalog. The executor should perform its own final
    /// authorization check before invoking a semantic operation.
    ///
    /// # Parameters
    /// - `definition`: Provider-facing schema approved for this branch.
    /// - `label`: Bounded display label for tool activity.
    /// - `execute`: Scriberig operation callback.
    ///
    /// # Returns
    /// An adapter implementing the upstream `AgentTool` contract.
    pub fn new<F, Fut>(definition: ToolDefinition, label: impl Into<String>, execute: F) -> Self
    where
        F: Fn(
                String,
                serde_json::Value,
                CancellationToken,
                Arc<dyn Fn(rpi_agent::ToolResultPartial) + Send + Sync>,
            ) -> Fut
            + Send
            + Sync
            + 'static,
        Fut: Future<Output = Result<rpi_agent::AgentToolResult, UpstreamAgentError>>
            + Send
            + 'static,
    {
        Self {
            schema: Tool {
                name: definition.name,
                description: definition.description,
                parameters: Schema(definition.parameters),
                constrained_sampling: None,
            },
            label: label.into(),
            execute: Arc::new(move |id, params, signal, on_update| {
                Box::pin(execute(id, params, signal, on_update))
            }),
        }
    }
}

#[async_trait]
impl AgentTool for SemanticAgentTool {
    /// Returns the provider-facing schema selected by Scriberig.
    ///
    /// # Returns
    /// The selected provider-facing schema.
    fn schema(&self) -> &Tool {
        &self.schema
    }

    /// Returns the safe activity label supplied by Scriberig.
    ///
    /// # Returns
    /// The safe activity label.
    fn label(&self) -> &str {
        &self.label
    }

    /// Serializes semantic operations until branch execution is concurrency-safe.
    ///
    /// # Returns
    /// The sequential tool execution mode.
    fn execution_mode(&self) -> ToolExecutionMode {
        ToolExecutionMode::Sequential
    }

    /// Executes the Scriberig callback after upstream schema validation.
    ///
    /// # Parameters
    /// - `tool_call_id`: Upstream correlation identity for this invocation.
    /// - `params`: Arguments validated against `schema` by `rpi-agent`.
    /// - `signal`: Cancellation token scoped to this tool execution.
    /// - `on_update`: Callback for bounded partial semantic results.
    ///
    /// # Returns
    /// The bounded provider-visible tool result.
    ///
    /// # Errors
    /// Returns the semantic executor's safe failure.
    async fn execute(
        &self,
        tool_call_id: &str,
        params: serde_json::Value,
        signal: CancellationToken,
        on_update: Arc<dyn Fn(rpi_agent::ToolResultPartial) + Send + Sync>,
    ) -> Result<rpi_agent::AgentToolResult, UpstreamAgentError> {
        (self.execute)(tool_call_id.to_owned(), params, signal, on_update).await
    }
}
