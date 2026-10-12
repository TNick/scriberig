//! Implements the non-streaming OpenAI-compatible provider adapter.

use serde::Serialize;
use std::sync::Arc;

use crate::config::ResolvedCredential;
use crate::provider::{
    FinishReason, ModelRequest, ModelResponse, ProviderAdapter, ProviderCapabilities,
    ProviderError, ProviderMetadata, ToolCall, ToolChoice, Usage,
};
use crate::provider::{MessageRole, ProviderConfig};
use openai_wire::WireResponse;
#[path = "openai_cancellation.rs"]
mod openai_cancellation;
#[path = "openai_transport.rs"]
mod openai_transport;
#[path = "openai_wire.rs"]
mod openai_wire;
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
}

impl OpenAiCompatibleProvider {
    /// Creates an adapter with the configured endpoint and timeouts.
    ///
    /// # Parameters
    /// - `config`: Provider configuration without an embedded secret.
    ///
    /// # Returns
    /// A configured OpenAI-compatible adapter.
    ///
    /// # Errors
    /// Returns a configuration error when the HTTP client cannot be built.
    pub fn new(config: ProviderConfig) -> Result<Self, ProviderError> {
        let client = reqwest::blocking::Client::builder()
            .connect_timeout(config.connect_timeout)
            .no_proxy()
            .http1_only()
            .user_agent("scriberig-agent-runtime/0.1")
            .build()
            .map_err(|error| ProviderError::Configuration(error.to_string()))?;
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
        })
    }

    /// Replaces the completion transport for deterministic adapter tests.
    ///
    /// # Parameters
    /// - `config`: Provider configuration without an embedded secret.
    /// - `transport`: In-process completion transport fixture.
    ///
    /// # Returns
    /// A configured provider using the supplied transport.
    ///
    /// # Errors
    /// Returns a configuration error when the backing HTTP client cannot be built.
    #[cfg(test)]
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
    /// # Returns
    /// A copy of the capability descriptor.
    #[must_use]
    pub const fn capabilities(&self) -> ProviderCapabilities {
        self.capabilities
    }

    /// Returns the configured endpoint with the chat-completions suffix.
    ///
    /// # Returns
    /// The endpoint URL used for requests.
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
    /// # Parameters
    /// - `request`: Provider-neutral model request.
    ///
    /// # Returns
    /// A provider-neutral model response.
    ///
    /// # Errors
    /// Returns a classified transport, HTTP, configuration, or parse failure.
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        let credential = self.config.resolve_credential()?;
        let payload = WireRequest::from_request(request, false);
        let response = self.transport.send(
            &self.endpoint(),
            &payload,
            credential.as_ref().map(ResolvedCredential::as_str),
            self.config.request_timeout,
        )?;
        let request_id = response.request_id;
        let status = response.status;
        if !status.is_success() {
            return Err(classify_status(status, request_id));
        }
        let wire: WireResponse = serde_json::from_slice(&response.body)
            .map_err(|error| ProviderError::MalformedResponse(error.to_string()))?;
        wire.into_response(request_id)
    }

    /// Completes through SSE while forwarding assistant text fragments.
    ///
    /// # Parameters
    /// - `request`: Provider-neutral request to execute.
    /// - `on_text`: Callback receiving provider-neutral text fragments.
    ///
    /// # Returns
    /// The assembled normalized response.
    ///
    /// # Errors
    /// Returns the classified provider failure.
    fn complete_streaming(
        &self,
        request: &ModelRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<ModelResponse, ProviderError> {
        self.complete_streaming_with(request, on_text)
    }
}

/// Serializes a provider-neutral request into the compatible wire subset.
#[derive(Serialize)]
struct WireRequest {
    model: String,
    messages: Vec<WireMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<WireTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "is_false")]
    stream: bool,
}

impl WireRequest {
    /// Converts a provider-neutral request into wire fields.
    ///
    /// # Parameters
    /// - `request`: Provider-neutral request.
    /// - `stream`: Whether the provider should stream SSE fragments.
    ///
    /// # Returns
    /// OpenAI-compatible request fields.
    fn from_request(request: &ModelRequest, stream: bool) -> Self {
        let tool_choice = match &request.tool_choice {
            ToolChoice::Auto => Some(serde_json::json!("auto")),
            ToolChoice::None => Some(serde_json::json!("none")),
            ToolChoice::Required(name) => Some(serde_json::json!({
                "type": "function",
                "function": {"name": name},
            })),
        };
        let response_format = request.response_schema.as_ref().map(|schema| {
            serde_json::json!({
                "type": "json_schema",
                "json_schema": {"name": "scriberig_output", "schema": schema},
            })
        });
        Self {
            model: request.model.clone(),
            messages: request
                .messages
                .iter()
                .map(WireMessage::from_message)
                .collect(),
            tools: request
                .tools
                .iter()
                .map(WireTool::from_definition)
                .collect(),
            tool_choice,
            response_format,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream,
        }
    }
}

/// Omits the default false streaming flag from ordinary requests.
#[allow(clippy::trivially_copy_pass_by_ref)]
///
/// # Parameters
/// - `value`: Streaming flag to inspect.
///
/// # Returns
/// `true` when the flag is false.
fn is_false(value: &bool) -> bool {
    !*value
}

/// Serializes one provider-neutral message.
#[derive(Serialize)]
struct WireMessage {
    role: &'static str,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

impl WireMessage {
    /// Converts a semantic message role to its wire spelling.
    ///
    /// # Parameters
    /// - `message`: Provider-neutral message.
    ///
    /// # Returns
    /// Wire-compatible message fields.
    fn from_message(message: &crate::provider::ModelMessage) -> Self {
        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
        };
        Self {
            role,
            content: message.content.clone(),
            tool_call_id: message.tool_call_id.clone(),
        }
    }
}

#[derive(Serialize)]
/// Serializes one function tool declaration.
struct WireTool {
    r#type: &'static str,
    function: WireFunction,
}

#[derive(Serialize)]
/// Serializes the function portion of a tool declaration.
struct WireFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

impl WireTool {
    /// Converts a trusted tool definition to a function declaration.
    ///
    /// # Parameters
    /// - `definition`: Trusted provider-neutral tool definition.
    ///
    /// # Returns
    /// Wire-compatible function tool fields.
    fn from_definition(definition: &crate::provider::ToolDefinition) -> Self {
        Self {
            r#type: "function",
            function: WireFunction {
                name: definition.name.clone(),
                description: definition.description.clone(),
                parameters: definition.parameters.clone(),
            },
        }
    }
}

impl WireResponse {
    /// Normalizes a compatible response without exposing wire DTOs.
    ///
    /// # Parameters
    /// - `self`: Wire response to normalize.
    /// - `request_id`: Safe provider request identifier.
    ///
    /// # Returns
    /// Normalized provider response.
    ///
    /// # Errors
    /// Returns malformed-response when choices or arguments are invalid.
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
                let arguments =
                    serde_json::from_str(&call.function.arguments).map_err(|error| {
                        ProviderError::MalformedResponse(format!(
                            "tool call arguments are not JSON: {error}"
                        ))
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
