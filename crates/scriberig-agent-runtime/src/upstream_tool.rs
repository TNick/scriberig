//! Adapts `ScribeRig`-approved tool schemas and executors to `rpi-agent`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use rpi_agent::{
    AgentError as UpstreamAgentError, AgentTool, AgentToolResult, TextContentOrImage,
    ToolExecutionMode,
};
use rpi_ai::types::Tool;
use scriberig_app::CapabilityContext;
use scriberig_core::EntityId;
use tokio_util::sync::CancellationToken;

use crate::tools::ToolCatalog;

const MAX_TOOL_RESULT_BYTES: usize = 16_384;

/// Future returned by a `ScribeRig` semantic tool executor.
pub type SemanticToolFuture =
    Pin<Box<dyn Future<Output = Result<rpi_agent::AgentToolResult, UpstreamAgentError>> + Send>>;

/// Executes one pre-authorized `ScribeRig` semantic tool through `rpi-agent`.
///
/// Tool exposure and capability filtering remain `ScribeRig`-owned. Construct
/// instances only from the branch-visible tool catalog.
///
/// # Public members
/// None.
///
/// # Private members
/// - `schema`: Upstream provider-facing tool schema.
/// - `label`: Safe label used by upstream tool events and presentation.
/// - `branch`: Canonical `ScribeRig` branch that owns this tool adapter.
/// - `catalog`: Trusted catalog used for final invocation authorization.
/// - `capabilities`: Capability grants snapshotted for this branch.
/// - `execute`: `ScribeRig` semantic operation executor.
pub struct SemanticAgentTool {
    schema: Tool,
    label: String,
    branch: EntityId,
    catalog: ToolCatalog,
    capabilities: CapabilityContext,
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
    /// Creates an upstream tool adapter from a `ScribeRig`-approved definition.
    ///
    /// The definition must be visible in the supplied branch catalog and
    /// capability snapshot. The executor must still authorize the concrete
    /// application operation before performing it. It must return only a safe,
    /// redacted semantic projection; this adapter bounds text and removes
    /// structured details but cannot inspect operation-specific secrets.
    /// For branch-producing tools, it must persist the `ScribeRig` branch result
    /// and return a compact projection containing its stable `ScribeRig` handle.
    ///
    /// # Parameters
    /// - `definition`: Provider-facing schema approved for this branch.
    /// - `label`: Bounded display label for tool activity.
    /// - `branch`: Canonical `ScribeRig` branch that owns this adapter.
    /// - `catalog`: Trusted catalog containing the definition.
    /// - `capabilities`: Capability grants for this branch.
    /// - `execute`: `ScribeRig` operation callback.
    ///
    /// # Returns
    /// An adapter implementing the upstream `AgentTool` contract.
    ///
    /// # Errors
    /// Returns an error when the tool definition is not visible to the branch.
    pub fn new<F, Fut>(
        definition: &Tool,
        label: impl Into<String>,
        branch: EntityId,
        catalog: ToolCatalog,
        capabilities: CapabilityContext,
        execute: F,
    ) -> Result<Self, String>
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
        if !catalog
            .descriptors_for(&capabilities)
            .iter()
            .any(|descriptor| descriptor.definition == *definition)
        {
            return Err("tool is not visible to this branch".to_owned());
        }

        Ok(Self {
            schema: definition.clone(),
            label: bounded_label(&label.into()),
            branch,
            catalog,
            capabilities,
            execute: Arc::new(move |id, params, signal, on_update| {
                Box::pin(execute(id, params, signal, on_update))
            }),
        })
    }

    /// Returns the canonical `ScribeRig` branch owning this adapter.
    ///
    /// # Returns
    /// The `ScribeRig` branch ID supplied at construction.
    pub(crate) fn branch(&self) -> EntityId {
        self.branch
    }
}

#[async_trait]
impl AgentTool for SemanticAgentTool {
    /// Returns the provider-facing schema selected by `ScribeRig`.
    ///
    /// # Returns
    /// The selected provider-facing schema.
    fn schema(&self) -> &Tool {
        &self.schema
    }

    /// Returns the safe activity label supplied by `ScribeRig`.
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

    /// Authorizes and executes the callback after upstream schema validation.
    ///
    /// # Parameters
    /// - `tool_call_id`: Upstream correlation identity for this invocation.
    /// - `params`: Arguments validated against `schema` by `rpi-agent`.
    /// - `signal`: Cancellation token scoped to this tool execution.
    /// - `on_update`: Callback for bounded partial semantic results.
    ///
    /// # Returns
    /// A bounded provider-visible projection of the semantic tool result.
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
        self.catalog
            .authorize_for_context(&self.schema.name, &self.capabilities)
            .map_err(|_| UpstreamAgentError::tool("tool is not authorized for this branch"))?;

        let on_update = Arc::new(move |update| on_update(bound_tool_result(update)));
        let result = (self.execute)(tool_call_id.to_owned(), params, signal, on_update)
            .await
            .map_err(|error| {
                if error.is_abort() {
                    error
                } else {
                    UpstreamAgentError::tool("semantic tool execution failed")
                }
            })?;
        Ok(bound_tool_result(result))
    }
}

/// Bounds tool labels before they enter upstream activity events.
///
/// # Parameters
/// - `label`: Trusted activity label supplied by the host.
///
/// # Returns
/// A control-free label of at most 128 Unicode scalar values.
fn bounded_label(label: &str) -> String {
    label
        .chars()
        .filter(|character| !character.is_control())
        .take(128)
        .collect()
}

/// Removes non-provider details and bounds the model-visible tool result.
///
/// Images, structured details, and dynamic tool additions are excluded so a
/// semantic executor cannot enlarge the branch's advertised tool set or
/// return an unbounded payload through upstream agent state.
///
/// # Parameters
/// - `result`: Semantic executor result to project.
///
/// # Returns
/// A text-only result bounded to `MAX_TOOL_RESULT_BYTES`.
///
/// # Errors
/// This projection cannot fail.
fn bound_tool_result(mut result: AgentToolResult) -> AgentToolResult {
    let mut text = String::with_capacity(MAX_TOOL_RESULT_BYTES);
    'blocks: for content in result.content {
        let TextContentOrImage::Text(content) = content else {
            continue;
        };
        for character in content.text.chars() {
            let width = character.len_utf8();
            if text.len() + width > MAX_TOOL_RESULT_BYTES {
                break 'blocks;
            }
            text.push(character);
        }
    }
    result.content = vec![TextContentOrImage::text(text)];
    result.details = serde_json::Value::Null;
    result.added_tool_names.clear();
    result
}
