//! Defines safe provider configuration and explicit capability metadata.

use std::fmt;
use std::time::Duration;

use super::provider::ProviderError;

/// Declares model-provider behavior available to the runtime.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProviderCapabilities {
    /// Whether response streaming is supported.
    pub streaming: bool,
    /// Whether structured tool calls are supported.
    pub tool_calls: bool,
    /// Whether multiple tool calls may be returned together.
    pub parallel_tool_calls: bool,
    /// Whether structured output schemas are supported.
    pub structured_output: bool,
    /// Whether token usage is reported.
    pub usage_reporting: bool,
    /// Whether provider request identifiers are returned.
    pub provider_request_id: bool,
    /// Whether provider model fingerprints are returned.
    pub provider_model_fingerprint: bool,
}

/// Selects known deviations from the OpenAI-compatible request family.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CompatibilityProfile {
    /// OpenAI-compatible `/chat/completions` semantics.
    #[default]
    ChatCompletions,
}

/// Selects an indirect source for a provider credential.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialSource {
    /// Read the credential from the named environment variable.
    Environment(String),
    /// Send no credential header.
    None,
}

/// Runtime provider configuration without an embedded secret value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderConfig {
    /// Safe profile identifier used in diagnostics.
    pub profile_id: String,
    /// Provider-compatible chat completion endpoint base URL.
    pub base_url: String,
    /// Provider model identifier.
    pub model: String,
    /// Indirect credential source.
    pub credential_source: CredentialSource,
    /// Maximum time allowed for one request.
    pub request_timeout: Duration,
    /// Maximum time allowed to establish a connection.
    pub connect_timeout: Duration,
    /// Whether streaming is enabled by default.
    pub stream_default: bool,
    /// Known wire compatibility profile.
    pub compatibility_profile: CompatibilityProfile,
}

impl ProviderConfig {
    /// Creates a configuration with conservative request timeouts.
    ///
    /// # Parameters
    /// - `profile_id`: Safe profile display identifier.
    /// - `base_url`: Provider endpoint base URL.
    /// - `model`: Provider model identifier.
    /// - `credential_source`: Indirect credential source.
    ///
    /// # Returns
    /// A provider configuration with streaming disabled.
    #[must_use]
    pub fn new(
        profile_id: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
        credential_source: CredentialSource,
    ) -> Self {
        Self {
            profile_id: profile_id.into(),
            base_url: base_url.into(),
            model: model.into(),
            credential_source,
            request_timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(10),
            stream_default: false,
            compatibility_profile: CompatibilityProfile::default(),
        }
    }

    /// Resolves the configured credential at request time.
    ///
    /// # Returns
    /// A secret wrapper when the configured source contains a value.
    ///
    /// # Errors
    /// Returns a configuration error when the environment source is missing
    /// or empty.
    pub fn resolve_credential(&self) -> Result<Option<ResolvedCredential>, ProviderError> {
        match &self.credential_source {
            CredentialSource::Environment(name) => {
                let value = std::env::var(name).map_err(|_| {
                    ProviderError::Configuration(format!(
                        "credential environment variable {name} is not set"
                    ))
                })?;
                if value.is_empty() {
                    return Err(ProviderError::Configuration(format!(
                        "credential environment variable {name} is empty"
                    )));
                }
                Ok(Some(ResolvedCredential(value)))
            }
            CredentialSource::None => Ok(None),
        }
    }
}

/// Opaque credential value resolved only inside the provider adapter.
pub struct ResolvedCredential(String);

impl ResolvedCredential {
    /// Returns the secret for constructing an outbound authorization header.
    ///
    /// # Returns
    /// The resolved credential value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ResolvedCredential {
    /// Redacts the credential in diagnostics.
    ///
    /// # Parameters
    /// - `self`: Credential to redact.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ResolvedCredential(REDACTED)")
    }
}

#[cfg(test)]
mod tests {
    use super::ResolvedCredential;

    /// Verifies credentials are never rendered in diagnostics.
    #[test]
    fn resolved_credentials_are_redacted() {
        let credential = ResolvedCredential("secret-value".to_owned());

        assert!(!format!("{credential:?}").contains("secret-value"));
    }
}
