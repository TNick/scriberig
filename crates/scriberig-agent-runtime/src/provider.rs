//! Defines provider-neutral model requests, responses, errors, and test fakes.

pub use super::config::{
    CompatibilityProfile, CredentialSource, ProviderCapabilities, ProviderConfig,
    ResolvedCredential,
};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex};

/// Identifies the semantic author of a model context message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MessageRole {
    /// System-level runtime instruction.
    System,
    /// Human-authored instruction.
    User,
    /// Model-authored response.
    Assistant,
    /// Result returned by an application tool.
    Tool,
}

/// One provider-neutral context message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelMessage {
    /// Semantic author of the message.
    pub role: MessageRole,
    /// Textual content supplied to the model.
    pub content: String,
    /// Tool call identifier for a tool result, when applicable.
    pub tool_call_id: Option<String>,
}

impl ModelMessage {
    /// Creates a user-authored message.
    ///
    /// # Parameters
    /// - `content`: Text supplied by the user.
    ///
    /// # Returns
    /// A provider-neutral user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_call_id: None,
        }
    }

    /// Creates a system-level message.
    ///
    /// # Parameters
    /// - `content`: Runtime instruction supplied to the model.
    ///
    /// # Returns
    /// A provider-neutral system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            tool_call_id: None,
        }
    }
}

/// Describes one trusted tool available to a model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDefinition {
    /// Stable provider-neutral tool identifier.
    pub name: String,
    /// Human-readable tool purpose.
    pub description: String,
    /// JSON schema for accepted arguments.
    pub parameters: serde_json::Value,
}

/// Controls whether the model may choose a tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolChoice {
    /// Let the model choose whether to call a tool.
    Auto,
    /// Require a final text response.
    None,
    /// Require the named tool.
    Required(String),
}

/// Provider-neutral model request.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelRequest {
    /// Provider model identifier.
    pub model: String,
    /// Ordered semantic context messages.
    pub messages: Vec<ModelMessage>,
    /// Trusted tools exposed to the model.
    pub tools: Vec<ToolDefinition>,
    /// Structured tool-selection policy.
    pub tool_choice: ToolChoice,
    /// Optional structured-output schema.
    pub response_schema: Option<serde_json::Value>,
    /// Optional sampling temperature.
    pub temperature: Option<f32>,
    /// Optional output token budget.
    pub max_tokens: Option<u32>,
    /// Safe request metadata for diagnostics and tracing.
    pub metadata: BTreeMap<String, String>,
}

impl ModelRequest {
    /// Creates a request with default generation policy.
    ///
    /// # Parameters
    /// - `model`: Provider model identifier.
    /// - `messages`: Ordered semantic context.
    ///
    /// # Returns
    /// A request with no tools or structured-output requirement.
    #[must_use]
    pub fn new(model: impl Into<String>, messages: Vec<ModelMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            tools: Vec::new(),
            tool_choice: ToolChoice::Auto,
            response_schema: None,
            temperature: None,
            max_tokens: None,
            metadata: BTreeMap::new(),
        }
    }
}

/// Explains why a normalized model response ended.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FinishReason {
    /// The model returned final text.
    Stop,
    /// The model requested one or more tools.
    ToolCall,
    /// The provider stopped generation at a length limit.
    Length,
    /// The provider returned a content-policy refusal.
    Refusal,
}

/// One normalized model tool call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolCall {
    /// Provider-neutral call identifier.
    pub id: String,
    /// Trusted tool identifier selected by the model.
    pub name: String,
    /// Parsed structured arguments.
    pub arguments: serde_json::Value,
}

/// Bounded token accounting returned by a provider.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Usage {
    /// Number of input tokens reported by the provider.
    pub prompt_tokens: u64,
    /// Number of generated tokens reported by the provider.
    pub completion_tokens: u64,
    /// Combined token count when reported.
    pub total_tokens: u64,
}

/// Safe provider metadata associated with a response.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProviderMetadata {
    /// Provider request identifier, when returned.
    pub request_id: Option<String>,
    /// Provider model fingerprint, when returned.
    pub model_fingerprint: Option<String>,
}

/// Normalized model response independent of provider wire formats.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelResponse {
    /// Final assistant text, when present.
    pub text: Option<String>,
    /// Structured tool calls requested by the model.
    pub tool_calls: Vec<ToolCall>,
    /// Normalized terminal reason.
    pub finish_reason: FinishReason,
    /// Optional usage accounting.
    pub usage: Option<Usage>,
    /// Safe provider metadata.
    pub metadata: ProviderMetadata,
}

/// Stable category for provider failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderError {
    /// Configuration is missing or invalid.
    Configuration(String),
    /// Credentials were rejected.
    Authentication(String),
    /// The provider denied the request.
    Permission(String),
    /// The provider rate-limited the request.
    RateLimited(String),
    /// The request exceeded a configured deadline.
    Timeout(String),
    /// The transport failed before a valid response arrived.
    Transport(String),
    /// The request was invalid for the selected provider.
    InvalidRequest(String),
    /// The provider response could not be normalized.
    MalformedResponse(String),
    /// The provider returned a server-side failure.
    ServerFailure(String),
    /// The caller cancelled the request.
    Cancelled,
}

impl fmt::Display for ProviderError {
    /// Formats a safe provider failure without credentials or request bodies.
    ///
    /// # Parameters
    /// - `self`: Provider failure to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (category, message) = match self {
            Self::Configuration(message) => ("configuration", message),
            Self::Authentication(message) => ("authentication", message),
            Self::Permission(message) => ("permission", message),
            Self::RateLimited(message) => ("rate_limited", message),
            Self::Timeout(message) => ("timeout", message),
            Self::Transport(message) => ("transport", message),
            Self::InvalidRequest(message) => ("invalid_request", message),
            Self::MalformedResponse(message) => ("malformed_response", message),
            Self::ServerFailure(message) => ("server_failure", message),
            Self::Cancelled => ("cancelled", &"request cancelled".to_owned()),
        };
        write!(formatter, "provider {category}: {message}")
    }
}

impl std::error::Error for ProviderError {}

/// Provider-neutral model adapter boundary.
pub trait ProviderAdapter: Send + Sync {
    /// Checks model output and API access through the normal credential path.
    ///
    /// # Parameters
    /// - `model`: Configured model identifier used for the check.
    ///
    /// # Returns
    /// Unit when the provider completes the request with the required marker.
    ///
    /// # Errors
    /// Returns the classified credential, transport, HTTP, or response failure.
    fn check_connection(&self, model: &str) -> Result<(), ProviderError> {
        let mut request = ModelRequest::new(
            model,
            vec![ModelMessage::user(
                "Connection check: reply only with `MODEL OK`. You may emit brief reasoning before the final answer; end with exactly `MODEL OK`.",
            )],
        );
        request.max_tokens = Some(512);
        request.tool_choice = ToolChoice::None;
        let response = self.complete(&request)?;
        if response.finish_reason == FinishReason::Refusal {
            return Err(ProviderError::MalformedResponse(
                "provider refused the connection check".to_owned(),
            ));
        }
        if response.finish_reason != FinishReason::Stop
            || !response
                .text
                .as_deref()
                .is_some_and(|text| text.trim_end().ends_with("MODEL OK"))
        {
            return Err(ProviderError::MalformedResponse(
                "provider did not finish the connection check with MODEL OK".to_owned(),
            ));
        }
        Ok(())
    }

    /// Completes one normalized model request.
    ///
    /// # Parameters
    /// - `request`: Provider-neutral request to execute.
    ///
    /// # Returns
    /// A normalized response independent of provider wire types.
    ///
    /// # Errors
    /// Returns a classified provider failure.
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError>;

    /// Completes a request while forwarding provider-neutral text fragments.
    ///
    /// # Parameters
    /// - `request`: Provider-neutral request to execute.
    /// - `on_text`: Callback receiving bounded assistant-text fragments.
    ///
    /// # Returns
    /// A normalized terminal response.
    ///
    /// # Errors
    /// Returns a classified provider failure.
    fn complete_streaming(
        &self,
        request: &ModelRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<ModelResponse, ProviderError> {
        let response = self.complete(request)?;
        if let Some(text) = &response.text {
            on_text(text);
        }
        Ok(response)
    }
}

/// Deterministic provider fake for runtime and integration tests.
#[derive(Clone, Debug)]
pub struct FakeProvider {
    responses: Arc<Mutex<VecDeque<Result<ModelResponse, ProviderError>>>>,
    requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl FakeProvider {
    /// Creates a fake with a finite scripted response queue.
    ///
    /// # Parameters
    /// - `responses`: Responses returned in invocation order.
    ///
    /// # Returns
    /// A fake provider that performs no network access.
    #[must_use]
    pub fn from_responses<I>(responses: I) -> Self
    where
        I: IntoIterator<Item = Result<ModelResponse, ProviderError>>,
    {
        Self {
            responses: Arc::new(Mutex::new(responses.into_iter().collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns a copy of all requests received by the fake.
    ///
    /// # Returns
    /// Requests in invocation order, or an empty list after lock recovery.
    #[must_use]
    pub fn requests(&self) -> Vec<ModelRequest> {
        self.requests
            .lock()
            .map(|requests| requests.clone())
            .unwrap_or_default()
    }
}

impl ProviderAdapter for FakeProvider {
    /// Records a request and returns the next scripted response.
    ///
    /// # Parameters
    /// - `request`: Provider-neutral request to record.
    ///
    /// # Returns
    /// The next scripted response.
    ///
    /// # Errors
    /// Returns the scripted provider failure or an exhausted-queue error.
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        if let Ok(mut requests) = self.requests.lock() {
            requests.push(request.clone());
        }
        self.responses
            .lock()
            .map_err(|_| ProviderError::Transport("fake response lock failed".to_owned()))?
            .pop_front()
            .unwrap_or_else(|| {
                Err(ProviderError::Transport(
                    "fake response queue is exhausted".to_owned(),
                ))
            })
    }
}
