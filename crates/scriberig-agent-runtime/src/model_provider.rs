//! Bridges normalized provider responses into the existing agent decision loop.

use std::collections::BTreeMap;

use crate::cache::{CachePolicy, ModelCache};
use crate::context::CompiledContext;
use crate::provider::{
    MessageRole, ModelMessage, ModelRequest, ProviderAdapter, ProviderError, ToolChoice,
};
use crate::tools::ToolCatalog;
use crate::{AgentMessage, ModelDecision};
use scriberig_agent_core::{ToolDetail, ToolInput};
use scriberig_app::CapabilityContext;

#[path = "model_provider/decision.rs"]
mod decision;
#[path = "model_provider/presentation.rs"]
mod presentation;
#[path = "model_provider/request_projection.rs"]
mod request_projection;

/// Adapts a normalized provider to the runtime's tool-loop contract.
pub struct ProviderModel<P> {
    /// Provider adapter receiving normalized requests.
    provider: P,
    /// Model identifier sent with each request.
    model: String,
    /// Thinking capability snapshotted for each request.
    thinking_level: Option<String>,
    /// Trusted tool definitions exposed to the provider.
    tools: Vec<crate::provider::ToolDefinition>,
    /// Optional trusted validator for returned tool calls.
    catalog: Option<ToolCatalog>,
    /// Capabilities available to this branch.
    capabilities: CapabilityContext,
    /// Optional complete-response cache and its policy.
    cache: Option<(ModelCache, CachePolicy)>,
    /// Monotonic local invocation sequence used for cache provenance.
    invocation_sequence: u64,
    /// Branch identity included in the cache key.
    cache_namespace: Option<String>,
    /// Bounded branch context compiled before the provider call.
    compiled_context: Option<CompiledContext>,
    /// Safe request projections awaiting consumption by `AgentRuntime`.
    request_projections: Vec<scriberig_session::ModelRequestProjection>,
}

impl<P> ProviderModel<P> {
    /// Creates a model adapter with no exposed tools.
    ///
    /// - `provider`: Normalized provider adapter.
    /// - `model`: Provider model identifier.
    ///
    /// A model provider suitable for `AgentRuntime`.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(provider: P, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
            thinking_level: None,
            tools: Vec::new(),
            catalog: None,
            capabilities: CapabilityContext::default(),
            cache: None,
            invocation_sequence: 0,
            cache_namespace: None,
            compiled_context: None,
            request_projections: Vec::new(),
        }
    }

    /// Adds the trusted tool catalog exposed to future requests.
    ///
    /// - `self`: Model adapter being configured.
    /// - `tools`: Provider-neutral trusted tool definitions.
    ///
    /// The configured model adapter.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_tools(mut self, tools: Vec<crate::provider::ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }

    /// Selects a provider-neutral thinking capability for future invocations.
    ///
    /// - `self`: Model adapter being configured.
    /// - `thinking_level`: Model-advertised capability identity.
    ///
    /// The configured model adapter.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_thinking_level(mut self, thinking_level: Option<String>) -> Self {
        self.thinking_level = thinking_level;
        self
    }

    /// Attaches the trusted catalog used to validate returned tool calls.
    ///
    /// - `self`: Model adapter being configured.
    /// - `catalog`: Trusted branch-visible tool catalog.
    ///
    /// The configured model adapter.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_catalog(mut self, catalog: ToolCatalog) -> Self {
        self.tools = catalog.definitions();
        self.catalog = Some(catalog);
        self
    }

    /// Restricts provider-visible tools to the branch capability context.
    ///
    /// - `self`: Model adapter being configured.
    /// - `capabilities`: Authority available to this branch.
    ///
    /// The configured model adapter.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_capabilities(mut self, capabilities: CapabilityContext) -> Self {
        self.capabilities = capabilities;
        if let Some(catalog) = &self.catalog {
            self.tools = catalog
                .descriptors_for(&self.capabilities)
                .into_iter()
                .map(|descriptor| descriptor.definition)
                .collect();
        }
        self
    }

    /// Installs a complete-response cache for exact request reuse.
    ///
    /// - `self`: Model adapter being configured.
    /// - `cache`: Shared normalized response cache.
    /// - `policy`: Cache policy for provider invocations.
    ///
    /// The configured model adapter.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_cache(mut self, cache: ModelCache, policy: CachePolicy) -> Self {
        self.cache = Some((cache, policy));
        self
    }

    /// Adds an isolated branch namespace to cache identity.
    ///
    /// - `self`: Model adapter being configured.
    /// - `namespace`: Stable branch identity.
    ///
    /// The adapter with branch-scoped cache keys.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_cache_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.cache_namespace = Some(namespace.into());
        self
    }

    /// Adds bounded compiled branch context to every normalized request.
    ///
    /// - `self`: Model adapter being configured.
    /// - `context`: Exact context selected for this branch.
    ///
    /// The adapter with provider-visible, cache-bound context.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_compiled_context(mut self, context: CompiledContext) -> Self {
        self.compiled_context = Some(context);
        self
    }
}

impl<P: ProviderAdapter> crate::ModelProvider for ProviderModel<P> {
    /// Converts runtime messages to a normalized request and response.
    ///
    /// - `messages`: Provider-neutral runtime conversation.
    ///
    /// A final response or validated inspect-tool decision.
    ///
    /// Returns provider failures or malformed tool arguments.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        self.decide_with_output(messages, &mut |_| {})
    }

    /// Converts runtime messages while forwarding provider text fragments.
    ///
    /// - `messages`: Provider-neutral runtime conversation.
    /// - `on_text`: Callback receiving streamed assistant text.
    ///
    /// A final response or validated inspect-tool decision.
    ///
    /// Returns provider failures or malformed tool arguments.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn decide_with_output(
        &mut self,
        messages: &[AgentMessage],
        on_text: &mut dyn FnMut(&str),
    ) -> Result<ModelDecision, String> {
        let request = self.model_request(messages);
        self.request_projections
            .push(request_projection::capture(&request));
        self.invocation_sequence += 1;
        let response = if let Some((cache, policy)) = self.cache.clone() {
            let fingerprint =
                blake3::hash(format!("{:?}\n{:?}", self.cache_namespace, request).as_bytes())
                    .to_hex()
                    .to_string();
            cache
                .execute(
                    policy,
                    fingerprint,
                    format!("model-invocation-{}", self.invocation_sequence),
                    || {
                        self.provider
                            .complete(&request)
                            .map_err(|error| provider_error(&error))
                    },
                )?
                .response
        } else {
            self.provider
                .complete_streaming(&request, on_text)
                .map_err(|error| provider_error(&error))?
        };
        let Some(call) = response.tool_calls.into_iter().next() else {
            return Ok(ModelDecision::Final(response.text.unwrap_or_default()));
        };
        let descriptor = if let Some(catalog) = &self.catalog {
            Some(
                catalog
                    .validate_for_context(&call, &self.capabilities)
                    .map_err(|error| error.to_string())?,
            )
        } else {
            None
        };
        if call.name != "inspect" {
            return decision::named_tool_decision(
                &self.tools,
                call,
                descriptor.and_then(|value| value.invocation_label.clone()),
                descriptor.map_or(
                    scriberig_session::ToolActivityKind::OpaqueFallback,
                    |value| value.category,
                ),
            );
        }
        let entity = call
            .arguments
            .get("entity")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "inspect tool requires an entity string".to_owned())?
            .parse()
            .map_err(|error| format!("invalid inspect entity: {error}"))?;
        let snapshot = call
            .arguments
            .get("snapshot")
            .and_then(serde_json::Value::as_str)
            .map(str::parse)
            .transpose()
            .map_err(|error| format!("invalid inspect snapshot: {error}"))?;
        let detail = match call
            .arguments
            .get("detail")
            .and_then(serde_json::Value::as_str)
        {
            Some("detailed") => ToolDetail::Detailed,
            _ => ToolDetail::Compact,
        };
        Ok(ModelDecision::Tool(ToolInput {
            entity,
            snapshot,
            detail,
        }))
    }

    /// Returns captured safe requests and clears the pending queue.
    ///
    /// Requests in the order submitted to the adapter.
    /// # Returns
    /// The operation result.
    fn take_request_projections(&mut self) -> Vec<scriberig_session::ModelRequestProjection> {
        std::mem::take(&mut self.request_projections)
    }
}

impl<P: ProviderAdapter> ProviderModel<P> {
    /// Builds the bounded normalized request for one runtime turn.
    ///
    /// - `messages`: Provider-neutral conversation messages.
    ///
    /// The model request with optional compiled context and configured tools.
    /// # Returns
    /// The operation result.
    fn model_request(&self, messages: &[AgentMessage]) -> ModelRequest {
        let mut request_messages =
            self.compiled_context
                .as_ref()
                .map_or_else(Vec::new, |context| {
                    vec![crate::provider::ModelMessage::system(format!(
                        "Compiled branch context:\n{}",
                        context.canonical
                    ))]
                });
        request_messages.extend(messages.iter().map(message));
        ModelRequest {
            model: self.model.clone(),
            thinking_level: self.thinking_level.clone(),
            messages: request_messages,
            tools: self.tools.clone(),
            tool_choice: ToolChoice::Auto,
            response_schema: None,
            temperature: None,
            max_tokens: None,
            metadata: BTreeMap::default(),
        }
    }
}

/// Converts one non-inspection provider tool call into a runtime decision.
///
/// - `call`: Validated provider tool call.
///
/// A delegated child request or named application tool request.
///
/// Returns an error when a child call omits required fields.
/// Reads a required string argument from a provider tool call.
///
/// - `call`: Provider tool call containing structured arguments.
/// - `name`: Required argument name.
///
/// The copied argument string.
///
/// Returns an error when the argument is missing or not a string.
/// Converts one runtime message into normalized provider context.
///
/// - `message`: Runtime message to convert.
///
/// A normalized provider context message.
/// # Returns
/// The operation result.
fn message(message: &AgentMessage) -> ModelMessage {
    match message {
        AgentMessage::User(content) => ModelMessage::user(content),
        AgentMessage::Tool(result) => ModelMessage {
            role: MessageRole::Tool,
            content: format!(
                "entity={} revision={:?} snapshot={:?} object_type={:?} summary={:?} handles={:?}",
                result.entity,
                result.revision,
                result.snapshot,
                result.object_type,
                result.summary,
                result.handles
            ),
            tool_call_id: None,
        },
        AgentMessage::Child(result) => ModelMessage {
            role: MessageRole::Tool,
            content: format!(
                "child_summary={} handles={:?}",
                result.summary, result.handles
            ),
            tool_call_id: None,
        },
    }
}

/// Converts a classified provider failure into the runtime error boundary.
///
/// - `error`: Classified provider failure.
///
/// Safe runtime error text.
/// # Returns
/// The operation result.
fn provider_error(error: &ProviderError) -> String {
    error.to_string()
}
