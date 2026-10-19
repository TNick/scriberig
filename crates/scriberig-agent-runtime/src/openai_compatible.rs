//! Implements the non-streaming OpenAI-compatible provider adapter.

use std::sync::Arc;

use crate::config::ResolvedCredential;
use crate::provider::ProviderConfig;
use crate::provider::{
    FinishReason, ModelRequest, ModelResponse, ProviderAdapter, ProviderCapabilities,
    ProviderError, ProviderMetadata, ToolCall, Usage,
};
use openai_wire::WireResponse;
#[path = "openai_cancellation.rs"]
mod openai_cancellation;
#[path = "openai_request.rs"]
mod openai_request;
#[path = "openai_transport.rs"]
mod openai_transport;
#[path = "openai_wire.rs"]
mod openai_wire;
use openai_request::WireRequest;
use openai_transport::{CompletionTransport, ReqwestCompletionTransport, classify_status};

/// OpenAI-compatible chat-completions provider.
#[derive(Clone)]
pub struct OpenAiCompatibleProvider {
    /// Runtime provider configuration.
    config: ProviderConfig,
    /// HTTP client with bounded connection behavior.
    client: reqwest::blocking::Client,
    /// Completion transport used by the normalized request path.
    transport: Arc<dyn CompletionTransport>,
    /// Declared capabilities for this compatibility adapter.
    capabilities: ProviderCapabilities,
    /// Adapter-local translation from public level ids to wire fields.
    thinking_parameters:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, serde_json::Value>>,
    /// Model-specific translation from public levels to wire fields.
    model_thinking_parameters: std::collections::BTreeMap<
        (String, String),
        std::collections::BTreeMap<String, serde_json::Value>,
    >,
}

impl OpenAiCompatibleProvider {
    /// Creates an adapter with the configured endpoint and timeouts.
    ///
    /// - `config`: Provider configuration without an embedded secret.
    ///
    /// A configured OpenAI-compatible adapter.
    ///
    /// Returns a configuration error when the HTTP client cannot be built.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        scriberig_secrets::validate_public_endpoint(&config.base_url).map_err(|_| {
            ProviderError::Configuration("invalid provider endpoint descriptor".to_owned())
        })?;
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(config.connect_timeout)
            .no_proxy()
            .http1_only()
            .user_agent("scriberig-agent-runtime/0.1")
            .build()
            .map_err(|_| {
                ProviderError::Configuration("provider client could not be initialized".to_owned())
            })?;
        let transport = Arc::new(ReqwestCompletionTransport::new(client.clone()));
        Ok(Self {
            config,
            client,
            transport,
            capabilities: ProviderCapabilities {
                tool_calls: true,
                streaming: true,
                usage_reporting: true,
                provider_request_id: true,
                provider_model_fingerprint: true,
                ..ProviderCapabilities::default()
            },
            thinking_parameters: std::collections::BTreeMap::new(),
            model_thinking_parameters: std::collections::BTreeMap::new(),
        })
    }

    /// Configures this adapter's translation for one advertised thinking id.
    ///
    /// - `self`: OpenAI-compatible provider adapter.
    /// - `level`: Provider-neutral thinking capability id.
    /// - `parameters`: Provider-specific request fields for this adapter.
    ///
    /// The adapter with the translation installed.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_thinking_parameters(
        mut self,
        level: impl Into<String>,
        parameters: std::collections::BTreeMap<String, serde_json::Value>,
    ) -> Self {
        self.thinking_parameters.insert(level.into(), parameters);
        self
    }

    /// Configures a model-specific thinking-level translation.
    ///
    /// - `self`: OpenAI-compatible provider adapter.
    /// - `model`: Provider-native model identifier.
    /// - `level`: Model-advertised thinking capability id.
    /// - `parameters`: Provider-specific request fields for this model level.
    ///
    /// The adapter with the model-specific translation installed.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_model_thinking_parameters(
        mut self,
        model: impl Into<String>,
        level: impl Into<String>,
        parameters: std::collections::BTreeMap<String, serde_json::Value>,
    ) -> Self {
        self.model_thinking_parameters
            .insert((model.into(), level.into()), parameters);
        self
    }

    /// Replaces the completion transport for deterministic adapter tests.
    ///
    /// - `config`: Provider configuration without an embedded secret.
    /// - `transport`: In-process completion transport fixture.
    ///
    /// A configured provider using the supplied transport.
    ///
    /// Returns a configuration error when the backing HTTP client cannot be built.
    #[cfg(test)]
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn with_transport(
        config: ProviderConfig,
        transport: Arc<dyn CompletionTransport>,
    ) -> Result<Self, ProviderError> {
        let mut provider = Self::new(config)?;
        provider.transport = transport;
        Ok(provider)
    }

    /// Returns the adapter's declared model capabilities.
    ///
    /// A copy of the capability descriptor.
    #[must_use]
    pub const fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities
    }

    /// Returns the configured endpoint with the chat-completions suffix.
    ///
    /// The endpoint URL used for requests.
    /// # Returns
    /// The operation result.
    fn endpoint(&self) -> String {
        format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        )
    }
}

impl ProviderAdapter for OpenAiCompatibleProvider {
    /// Sends a normalized request and parses one normalized response.
    ///
    /// - `request`: Provider-neutral model request.
    ///
    /// A provider-neutral model response.
    ///
    /// Returns a classified transport, HTTP, configuration, or parse failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        let credential = self.config.resolve_credential()?;
        let payload = WireRequest::from_request_for_models(
            request,
            false,
            &self.thinking_parameters,
            &self.model_thinking_parameters,
        );
        let response = self.transport.send(
            &self.endpoint(),
            &payload,
            credential.as_ref().map(ResolvedCredential::as_str),
            self.config.request_timeout,
        )?;
        let request_id = response.request_id.filter(|id| {
            credential
                .as_ref()
                .is_none_or(|secret| !id.contains(secret.as_str()))
        });
        let status = response.status;
        if !status.is_success() {
            return Err(classify_status(status, request_id));
        }
        let wire: WireResponse = serde_json::from_slice(&response.body).map_err(|_| {
            ProviderError::MalformedResponse("invalid provider response".to_owned())
        })?;
        let mut normalized = wire.into_response(request_id)?;
        crate::credential_redaction::redact_response(
            &mut normalized,
            credential.as_ref().map(ResolvedCredential::as_str),
        );
        Ok(normalized)
    }

    /// Completes through SSE while forwarding assistant text fragments.
    ///
    /// - `request`: Provider-neutral request to execute.
    /// - `on_text`: Callback receiving provider-neutral text fragments.
    ///
    /// The assembled normalized response.
    ///
    /// Returns the classified provider failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn complete_streaming(
        &self,
        request: &ModelRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<ModelResponse, ProviderError> {
        self.complete_streaming_with(request, on_text)
    }
}

impl WireResponse {
    /// Normalizes a compatible response without exposing wire DTOs.
    ///
    /// - `self`: Wire response to normalize.
    /// - `request_id`: Safe provider request identifier.
    ///
    /// Normalized provider response.
    ///
    /// Returns malformed-response when choices or arguments are invalid.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn into_response(self, request_id: Option<String>) -> Result<ModelResponse, ProviderError> {
        let choice = self.choices.into_iter().next().ok_or_else(|| {
            ProviderError::MalformedResponse("response has no choices".to_owned())
        })?;
        let tool_calls = choice
            .message
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .map(|call| {
                let arguments = serde_json::from_str(&call.function.arguments).map_err(|_| {
                    ProviderError::MalformedResponse("tool call arguments are not JSON".to_owned())
                })?;
                Ok(ToolCall {
                    id: call.id,
                    name: call.function.name,
                    arguments,
                })
            })
            .collect::<Result<Vec<_>, ProviderError>>()?;
        let finish_reason = match choice.finish_reason.as_deref() {
            Some("tool_calls") => FinishReason::ToolCall,
            Some("length") => FinishReason::Length,
            Some("content_filter") => FinishReason::Refusal,
            _ => FinishReason::Stop,
        };
        Ok(ModelResponse {
            text: choice.message.content,
            tool_calls,
            finish_reason,
            usage: self.usage.map(|usage| Usage {
                prompt_tokens: usage.input,
                completion_tokens: usage.output,
                total_tokens: usage.total,
            }),
            metadata: ProviderMetadata {
                request_id,
                model_fingerprint: self.model,
            },
        })
    }
}

#[cfg(test)]
#[path = "openai_compatible_tests.rs"]
mod tests;
