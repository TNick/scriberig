//! Bridges normalized provider responses into the existing agent decision loop.

use std::collections::BTreeMap;

use crate::cache::{CachePolicy, ModelCache};
use crate::provider::{
    MessageRole, ModelMessage, ModelRequest, ProviderAdapter, ProviderError, ToolChoice,
};
use crate::tools::ToolCatalog;
use crate::{AgentMessage, ModelDecision};
use scriberig_agent_core::{ToolDetail, ToolInput};
use scriberig_app::CapabilityContext;

/// Adapts a normalized provider to the runtime's tool-loop contract.
pub struct ProviderModel<P> {
    /// Provider adapter receiving normalized requests.
    provider: P,
    /// Model identifier sent with each request.
    model: String,
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
}

impl<P> ProviderModel<P> {
    /// Creates a model adapter with no exposed tools.
    ///
    /// # Parameters
    /// - `provider`: Normalized provider adapter.
    /// - `model`: Provider model identifier.
    ///
    /// # Returns
    /// A model provider suitable for `AgentRuntime`.
    #[must_use]
    pub fn new(provider: P, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
            tools: Vec::new(),
            catalog: None,
            capabilities: CapabilityContext::default(),
            cache: None,
            invocation_sequence: 0,
            cache_namespace: None,
        }
    }

    /// Adds the trusted tool catalog exposed to future requests.
    ///
    /// # Parameters
    /// - `self`: Model adapter being configured.
    /// - `tools`: Provider-neutral trusted tool definitions.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_tools(mut self, tools: Vec<crate::provider::ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }

    /// Attaches the trusted catalog used to validate returned tool calls.
    ///
    /// # Parameters
    /// - `self`: Model adapter being configured.
    /// - `catalog`: Trusted branch-visible tool catalog.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_catalog(mut self, catalog: ToolCatalog) -> Self {
        self.tools = catalog.definitions();
        self.catalog = Some(catalog);
        self
    }

    /// Restricts provider-visible tools to the branch capability context.
    ///
    /// # Parameters
    /// - `self`: Model adapter being configured.
    /// - `capabilities`: Authority available to this branch.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
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
    /// # Parameters
    /// - `self`: Model adapter being configured.
    /// - `cache`: Shared normalized response cache.
    /// - `policy`: Cache policy for provider invocations.
    ///
    /// # Returns
    /// The configured model adapter.
    #[must_use]
    pub fn with_cache(mut self, cache: ModelCache, policy: CachePolicy) -> Self {
        self.cache = Some((cache, policy));
        self
    }

    /// Adds an isolated branch namespace to cache identity.
    ///
    /// # Parameters
    /// - `self`: Model adapter being configured.
    /// - `namespace`: Stable branch identity.
    ///
    /// # Returns
    /// The adapter with branch-scoped cache keys.
    #[must_use]
    pub fn with_cache_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.cache_namespace = Some(namespace.into());
        self
    }
}

impl<P: ProviderAdapter> crate::ModelProvider for ProviderModel<P> {
    /// Converts runtime messages to a normalized request and response.
    ///
    /// # Parameters
    /// - `messages`: Provider-neutral runtime conversation.
    ///
    /// # Returns
    /// A final response or validated inspect-tool decision.
    ///
    /// # Errors
    /// Returns provider failures or malformed tool arguments.
    fn decide(&mut self, messages: &[AgentMessage]) -> Result<ModelDecision, String> {
        self.decide_with_output(messages, &mut |_| {})
    }

    /// Converts runtime messages while forwarding provider text fragments.
    ///
    /// # Parameters
    /// - `messages`: Provider-neutral runtime conversation.
    /// - `on_text`: Callback receiving streamed assistant text.
    ///
    /// # Returns
    /// A final response or validated inspect-tool decision.
    ///
    /// # Errors
    /// Returns provider failures or malformed tool arguments.
    fn decide_with_output(
        &mut self,
        messages: &[AgentMessage],
        on_text: &mut dyn FnMut(&str),
    ) -> Result<ModelDecision, String> {
        let request = ModelRequest {
            model: self.model.clone(),
            messages: messages.iter().map(message).collect(),
            tools: self.tools.clone(),
            tool_choice: ToolChoice::Auto,
            response_schema: None,
            temperature: None,
            max_tokens: None,
            metadata: BTreeMap::default(),
        };
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
        if let Some(catalog) = &self.catalog {
            catalog
                .validate_for_context(&call, &self.capabilities)
                .map_err(|error| error.to_string())?;
        }
        if call.name != "inspect" {
            if call.name == "delegate" {
                let purpose = call
                    .arguments
                    .get("purpose")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| "delegate tool requires a purpose".to_owned())?;
                let prompt = call
                    .arguments
                    .get("prompt")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| "delegate tool requires a prompt".to_owned())?;
                return Ok(ModelDecision::Child(crate::ChildRequest {
                    purpose: purpose.to_owned(),
                    prompt: prompt.to_owned(),
                }));
            }
            return Ok(ModelDecision::NamedTool {
                name: call.name,
                arguments: call.arguments,
            });
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
}

/// Converts one runtime message into normalized provider context.
///
/// # Parameters
/// - `message`: Runtime message to convert.
///
/// # Returns
/// A normalized provider context message.
fn message(message: &AgentMessage) -> ModelMessage {
    match message {
        AgentMessage::User(content) => ModelMessage::user(content),
        AgentMessage::Tool(result) => ModelMessage {
            role: MessageRole::Tool,
            content: format!(
                "entity={} revision={:?} snapshot={:?} summary={:?} handles={:?}",
                result.entity, result.revision, result.snapshot, result.summary, result.handles
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
/// # Parameters
/// - `error`: Classified provider failure.
///
/// # Returns
/// Safe runtime error text.
fn provider_error(error: &ProviderError) -> String {
    error.to_string()
}
