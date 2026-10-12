//! Implements HTTP transport and SSE streaming for the compatible provider.

use std::io::{BufRead, BufReader};

use crate::provider::{ModelRequest, ModelResponse, ProviderError};
use crate::stream::StreamAssembler;

use super::OpenAiCompatibleProvider;

impl OpenAiCompatibleProvider {
    /// Sends one streaming request and assembles its complete SSE response.
    ///
    /// # Parameters
    /// - `self`: Configured provider adapter.
    /// - `request`: Provider-neutral request to execute.
    ///
    /// # Returns
    /// A normalized response assembled from all stream fragments.
    ///
    /// # Errors
    /// Returns a classified transport, HTTP, configuration, or stream failure.
    pub fn complete_streaming(
        &self,
        request: &ModelRequest,
    ) -> Result<ModelResponse, ProviderError> {
        let credential = self.config.resolve_credential()?;
        let payload = super::WireRequest::from_request(request, true);
        let mut builder = self
            .client
            .post(self.endpoint())
            .timeout(self.config.request_timeout)
            .json(&payload);
        if let Some(credential) = credential {
            builder = builder.bearer_auth(credential.as_str());
        }
        let response = builder.send().map_err(|error| classify_transport(&error))?;
        let request_id = response
            .headers()
            .get("x-request-id")
            .or_else(|| response.headers().get("request-id"))
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let status = response.status();
        if !status.is_success() {
            return Err(classify_status(status, request_id));
        }
        let mut assembler = StreamAssembler::new();
        for line in BufReader::new(response).lines() {
            let line = line.map_err(|error| ProviderError::Transport(error.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            if assembler.push_sse_line(&line)? {
                break;
            }
        }
        let mut normalized = assembler.finish()?;
        normalized.metadata.request_id = request_id;
        Ok(normalized)
    }
}

/// Classifies a safe transport failure.
///
/// # Parameters
/// - `super`: Parent provider module namespace.
/// - `error`: Transport error returned by the HTTP client.
///
/// # Returns
/// A provider-neutral transport category.
pub(super) fn classify_transport(error: &reqwest::Error) -> ProviderError {
    if error.is_timeout() {
        ProviderError::Timeout(format!("provider request timed out: {error}"))
    } else if error.is_builder() {
        ProviderError::Configuration(format!("provider request could not be built: {error}"))
    } else {
        ProviderError::Transport(format!("provider request failed: {error}"))
    }
}

/// Classifies an HTTP status without retaining a sensitive response body.
///
/// # Parameters
/// - `super`: Parent provider module namespace.
/// - `status`: HTTP status returned by the provider.
/// - `request_id`: Safe provider request identifier.
///
/// # Returns
/// A provider-neutral HTTP failure category.
pub(super) fn classify_status(
    status: reqwest::StatusCode,
    request_id: Option<String>,
) -> ProviderError {
    let suffix = request_id
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
