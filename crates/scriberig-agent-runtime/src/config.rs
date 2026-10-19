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
#[derive(Clone)]
pub enum CredentialSource {
    /// Read the credential from the named environment variable.
    Environment(String),
    /// Resolve through an injected destination-authorized secret boundary.
    Authorized(std::sync::Arc<dyn scriberig_secrets::CredentialProvider>),
    /// Send no credential header.
    None,
}

/// Runtime provider configuration without an embedded secret value.
#[derive(Clone, Debug)]
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
    /// - `profile_id`: Safe profile display identifier.
    /// - `base_url`: Provider endpoint base URL.
    /// - `model`: Provider model identifier.
    /// - `credential_source`: Indirect credential source.
    ///
    /// A provider configuration with streaming disabled.
    #[must_use]
    /// # Returns
    /// The operation result.
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
    /// A secret wrapper when the configured source contains a value.
    ///
    /// Returns a configuration error when the environment source is missing
    /// or empty.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn resolve_credential(&self) -> Result<Option<ResolvedCredential>, ProviderError> {
        let value = match &self.credential_source {
            CredentialSource::Environment(name) => resolve_environment(name, &self.base_url),
            CredentialSource::Authorized(provider) => provider.credential(&self.base_url),
            CredentialSource::None => Ok(None),
        }
        .map_err(|error| ProviderError::Configuration(error.to_string()))?;
        Ok(value.map(ResolvedCredential))
    }
}

impl fmt::Debug for CredentialSource {
    /// Prints source kind without credential-provider internals.
    ///
    /// - `formatter`: Output formatter.
    ///
    /// Formatting success.
    ///
    /// Returns formatter failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::None => "None",
            Self::Environment(_) => "Environment([reference])",
            Self::Authorized(_) => "Authorized([reference])",
        })
    }
}

/// Resolves a legacy environment reference through live policy enforcement.
///
/// - `name`: Environment variable reference.
/// - `endpoint`: Actual provider destination.
///
/// An authorized protected credential.
///
/// Returns safe policy or source failures.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn resolve_environment(
    name: &str,
    endpoint: &str,
) -> Result<Option<scriberig_secrets::SecretValue>, scriberig_secrets::SecretError> {
    use scriberig_secrets::{CredentialProvider, ExecutionMode, SecretUseContext};
    let provider = scriberig_secrets::registry::AuthorizedCredential {
        reference: scriberig_secrets::SecretRef::Environment {
            name: name.to_owned(),
        },
        context: SecretUseContext {
            project_root: std::env::current_dir()
                .map_err(|_| scriberig_secrets::SecretError::Denied)?,
            purpose: "model_provider".to_owned(),
            mode: ExecutionMode::Interactive,
        },
        locations: scriberig_secrets::registry::RegistryLocations::from_process()?,
        invocation_binding: None,
        resolver: std::sync::Arc::new(scriberig_secrets::SourceResolver),
    };
    provider.credential(endpoint)
}

/// Opaque credential value resolved only inside the provider adapter.
pub struct ResolvedCredential(scriberig_secrets::SecretValue);

impl ResolvedCredential {
    /// Returns the secret for constructing an outbound authorization header.
    ///
    /// The resolved credential value.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &str {
        self.0.expose()
    }
}

impl fmt::Debug for ResolvedCredential {
    /// Redacts the credential in diagnostics.
    ///
    /// - `self`: Credential to redact.
    /// - `formatter`: Destination formatter.
    ///
    /// Formatting success or the formatter error.
    ///
    /// Returns the formatter error when output cannot be written.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
        let credential = ResolvedCredential(
            scriberig_secrets::SecretValue::new("secret-value".to_owned()).expect("valid secret"),
        );

        assert!(!format!("{credential:?}").contains("secret-value"));
    }
}
