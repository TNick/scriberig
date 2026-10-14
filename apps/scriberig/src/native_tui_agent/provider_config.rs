//! Resolves the native agent provider from trusted process configuration.

use scriberig_agent_runtime::provider::{CredentialSource, ProviderConfig, ProviderError};

use crate::native_provider::NativeProvider;
use crate::self_hosting_provider::{CancellationPhase, ValidationMode};

/// Builds one configured provider adapter for the TUI runtime.
///
/// # Parameters
/// - `provider_kind`: Trusted provider selection from process configuration.
/// - `model`: Configured model name.
/// - `super`: Parent native agent runtime module.
///
/// # Returns
/// Configured provider, or `None` when remote configuration is incomplete.
///
/// # Errors
/// Returns a structured error for an unsupported provider or scenario.
pub(super) fn from_environment(
    provider_kind: &str,
    model: &str,
) -> Result<Option<NativeProvider>, ProviderError> {
    match provider_kind {
        "deterministic" => deterministic_provider(),
        "openai_compatible" => std::env::var("SCRIBE_AGENT_BASE_URL")
            .ok()
            .map(|base_url| {
                let credential = std::env::var("SCRIBE_AGENT_API_KEY_ENV")
                    .map_or(CredentialSource::None, CredentialSource::Environment);
                let config = ProviderConfig::new("local", base_url, model.to_owned(), credential);
                scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider::new(config)
                    .map(NativeProvider::OpenAiCompatible)
            })
            .transpose(),
        unknown => Err(ProviderError::Configuration(format!(
            "unsupported SCRIBE_AGENT_PROVIDER: {unknown}"
        ))),
    }
}

/// Selects a registered deterministic provider scenario.
///
/// # Returns
/// Configured deterministic scenario.
///
/// # Errors
/// Returns a configuration error when the scenario identifier is unknown.
fn deterministic_provider() -> Result<Option<NativeProvider>, ProviderError> {
    let scenario =
        std::env::var("SCRIBE_AGENT_SCENARIO").unwrap_or_else(|_| "bootstrap".to_owned());
    let provider = match scenario.as_str() {
        "bootstrap" => {
            NativeProvider::Deterministic(crate::deterministic_provider::BootstrapProvider)
        }
        "bootstrap_import_failure" => NativeProvider::BootstrapImportFailure(
            crate::bootstrap_failure_provider::BootstrapImportFailureProvider,
        ),
        "self_hosting_php" => self_hosting(ValidationMode::Normal, None),
        "self_hosting_php_validation_failure" => self_hosting(ValidationMode::FailFirst, None),
        "self_hosting_php_validation_repair" => self_hosting(ValidationMode::RepairFirst, None),
        "self_hosting_php_cancellation" => {
            self_hosting(ValidationMode::Normal, Some(CancellationPhase::Onboarding))
        }
        "self_hosting_php_discovery_cancellation" => {
            self_hosting(ValidationMode::Normal, Some(CancellationPhase::Discovery))
        }
        unknown => {
            return Err(ProviderError::Configuration(format!(
                "unsupported SCRIBE_AGENT_SCENARIO: {unknown}"
            )));
        }
    };
    Ok(Some(provider))
}

/// Builds a PHP self-hosting scenario with explicit validation and cancellation behavior.
///
/// # Parameters
/// - `validation`: First-candidate validation behavior.
/// - `cancellation`: Optional delayed operation phase.
///
/// # Returns
/// Provider-neutral deterministic PHP scenario.
fn self_hosting(
    validation: ValidationMode,
    cancellation: Option<CancellationPhase>,
) -> NativeProvider {
    NativeProvider::SelfHostingPhp(crate::self_hosting_provider::PhpSelfHostingProvider {
        validation,
        cancellation,
    })
}
