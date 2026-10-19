//! Implements HTTP transport and SSE streaming for the compatible provider.

use std::io::{BufRead, BufReader};
use std::time::Duration;

use crate::provider::{ModelRequest, ModelResponse, ProviderError};
use crate::stream::StreamAssembler;

use super::OpenAiCompatibleProvider;

/// Response returned by one compatible completion transport.
pub(super) struct TransportResponse {
    /// HTTP status returned by the transport.
    pub(super) status: reqwest::StatusCode,
    /// Safe request identifier extracted from response headers.
    pub(super) request_id: Option<String>,
    /// Response body bytes.
    pub(super) body: Vec<u8>,
}

/// Sends one non-streaming compatible completion request.
pub(super) trait CompletionTransport: Send + Sync {
    /// Sends a JSON completion request through the transport boundary.
    ///
    /// - `endpoint`: Complete chat-completions URL.
    /// - `payload`: Serialized provider-neutral request.
    /// - `credential`: Optional bearer credential.
    /// - `timeout`: Maximum request duration.
    ///
    /// A normalized HTTP response envelope.
    ///
    /// Returns a classified transport failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn send(
        &self,
        endpoint: &str,
        payload: &super::WireRequest,
        credential: Option<&str>,
        timeout: Duration,
    ) -> Result<TransportResponse, ProviderError>;
}

/// Uses reqwest for production-compatible completion requests.
pub(super) struct ReqwestCompletionTransport {
    /// HTTP client configured by the provider boundary.
    client: reqwest::blocking::Client,
}

impl ReqwestCompletionTransport {
    /// Creates a reqwest-backed completion transport.
    ///
    /// - `super`: Parent module visibility boundary.
    /// - `client`: Configured reqwest client.
    ///
    /// A production completion transport.
    /// # Returns
    /// The operation result.
    pub(super) fn new(client: reqwest::blocking::Client) -> Self {
        Self { client }
    }
}

impl CompletionTransport for ReqwestCompletionTransport {
    /// Sends one completion request and buffers its response body.
    ///
    /// - `endpoint`: Complete chat-completions URL.
    /// - `payload`: Serialized provider-neutral request.
    /// - `credential`: Optional bearer credential.
    /// - `timeout`: Maximum request duration.
    ///
    /// The HTTP response envelope.
    ///
    /// Returns a classified reqwest failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn send(
        &self,
        endpoint: &str,
        payload: &super::WireRequest,
        credential: Option<&str>,
        timeout: Duration,
    ) -> Result<TransportResponse, ProviderError> {
        let mut builder = self.client.post(endpoint).timeout(timeout).json(payload);
        if let Some(credential) = credential {
            builder = builder.bearer_auth(credential);
        }
        let response = builder.send().map_err(|error| classify_transport(&error))?;
        let request_id = response
            .headers()
            .get("x-request-id")
            .or_else(|| response.headers().get("request-id"))
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let request_id =
            request_id.filter(|id| credential.is_none_or(|secret| !id.contains(secret)));
        let status = response.status();
        let body = response
            .bytes()
            .map_err(|_| {
                ProviderError::Transport("provider response could not be read".to_owned())
            })?
            .to_vec();
        Ok(TransportResponse {
            status,
            request_id,
            body,
        })
    }
}

impl OpenAiCompatibleProvider {
    /// Sends one streaming request and assembles its complete SSE response.
    ///
    /// - `self`: Configured provider adapter.
    /// - `request`: Provider-neutral request to execute.
    ///
    /// A normalized response assembled from all stream fragments.
    ///
    /// Returns a classified transport, HTTP, configuration, or stream failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_streaming(
        &self,
        request: &ModelRequest,
    ) -> Result<ModelResponse, ProviderError> {
        self.complete_streaming_with(request, &mut |_| {})
    }

    /// Sends one streaming request and forwards each assistant text fragment.
    ///
    /// - `self`: Configured provider adapter.
    /// - `request`: Provider-neutral request to execute.
    /// - `on_text`: Callback receiving bounded provider text fragments.
    ///
    /// A normalized response assembled from all stream fragments.
    ///
    /// Returns a classified transport, HTTP, configuration, or stream failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn complete_streaming_with(
        &self,
        request: &ModelRequest,
        on_text: &mut dyn FnMut(&str),
    ) -> Result<ModelResponse, ProviderError> {
        let credential = self.config.resolve_credential()?;
        let payload = super::WireRequest::from_request_for_models(
            request,
            true,
            &self.thinking_parameters,
            &self.model_thinking_parameters,
        );
        let mut builder = self
            .client
            .post(self.endpoint())
            .timeout(self.config.request_timeout)
            .json(&payload);
        if let Some(credential) = &credential {
            builder = builder.bearer_auth(credential.as_str());
        }
        let response = builder.send().map_err(|error| classify_transport(&error))?;
        let request_id = response
            .headers()
            .get("x-request-id")
            .or_else(|| response.headers().get("request-id"))
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let secret = credential
            .as_ref()
            .map(crate::config::ResolvedCredential::as_str);
        let request_id = request_id.filter(|id| secret.is_none_or(|value| !id.contains(value)));
        let status = response.status();
        if !status.is_success() {
            return Err(classify_status(status, request_id));
        }
        let mut assembler = StreamAssembler::new();
        let mut redactor = crate::credential_redaction::DeltaRedactor::new(secret);
        for line in BufReader::new(response).lines() {
            let line = line.map_err(|_| {
                ProviderError::Transport("provider stream could not be read".to_owned())
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let before = assembler.text_len();
            if assembler.push_sse_line(&line)? {
                break;
            }
            if let Some(delta) = assembler.text_since(before) {
                let safe = redactor.push(delta);
                if !safe.is_empty() {
                    on_text(&safe);
                }
            }
        }
        let mut normalized = assembler.finish()?;
        normalized.metadata.request_id = request_id;
        let suffix = redactor.finish();
        if !suffix.is_empty() {
            on_text(&suffix);
        }
        crate::credential_redaction::redact_response(&mut normalized, secret);
        Ok(normalized)
    }
}

/// Classifies a safe transport failure.
///
/// - `super`: Parent provider module namespace.
/// - `error`: Transport error returned by the HTTP client.
///
/// A provider-neutral transport category.
/// # Returns
/// The operation result.
pub(super) fn classify_transport(error: &reqwest::Error) -> ProviderError {
    if error.is_timeout() {
        ProviderError::Timeout("provider request timed out".to_owned())
    } else if error.is_builder() {
        ProviderError::Configuration("provider request could not be built".to_owned())
    } else {
        ProviderError::Transport("provider request failed".to_owned())
    }
}

/// Classifies an HTTP status without retaining a sensitive response body.
///
/// - `super`: Parent provider module namespace.
/// - `status`: HTTP status returned by the provider.
/// - `request_id`: Safe provider request identifier.
///
/// A provider-neutral HTTP failure category.
/// # Returns
/// The operation result.
pub(super) fn classify_status(
    status: reqwest::StatusCode,
    request_id: Option<String>,
) -> ProviderError {
    let suffix = request_id
        .filter(|id| {
            id.len() <= 128
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
        })
        .map(|id| format!(" (request {id})"))
        .unwrap_or_default();
    let message = format!("HTTP status {status}{suffix}");
    match status.as_u16() {
        401 | 403 => ProviderError::Authentication(message),
        408 | 504 => ProviderError::Timeout(message),
        429 => ProviderError::RateLimited(message),
        400..=499 => ProviderError::InvalidRequest(message),
        _ => ProviderError::ServerFailure(message),
    }
}
