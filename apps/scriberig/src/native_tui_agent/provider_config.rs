//! Resolves the native agent provider from trusted process configuration.

use scriberig_agent_runtime::provider::{CredentialSource, ProviderConfig, ProviderError};

use crate::native_provider::NativeProvider;
use crate::self_hosting_provider::{CancellationPhase, ValidationMode};
#[path = "provider_config/catalog.rs"]
pub(super) mod catalog;

/// Builds one configured provider adapter for the TUI runtime.
///
/// - `provider_kind`: Trusted provider selection from process configuration.
/// - `model`: Configured model name.
/// - `catalog`: Safe model descriptors and provider-private thinking mappings.
/// - `super`: Parent native agent runtime module.
///
/// Configured provider, or `None` when remote configuration is incomplete.
///
/// Returns a structured error for an unsupported provider or scenario.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn from_environment(
    provider_kind: &str,
    model: &str,
    catalog: &catalog::ProviderCatalog,
) -> Result<Option<NativeProvider>, ProviderError> {
    match provider_kind {
        "deterministic" => deterministic_provider(),
        "openai_compatible" => std::env::var("SCRIBE_AGENT_BASE_URL")
            .ok()
            .map(|base_url| {
                let credential = std::env::var("SCRIBE_AGENT_API_KEY_ENV")
                    .map_or(CredentialSource::None, CredentialSource::Environment);
                let config = ProviderConfig::new("local", base_url, model.to_owned(), credential);
                let mut provider =
                    scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider::new(
                        config,
                    )?;
                for (model, level, parameters) in &catalog.thinking_parameters {
                    provider = provider.with_model_thinking_parameters(
                        model.clone(),
                        level.clone(),
                        parameters.clone(),
                    );
                }
                Ok(NativeProvider::OpenAiCompatible(Box::new(provider)))
            })
            .transpose(),
        unknown => Err(ProviderError::Configuration(format!(
            "unsupported SCRIBE_AGENT_PROVIDER: {unknown}"
        ))),
    }
}

/// Builds one named OpenAI-compatible adapter from credential-free references.
///
/// - `connection`: Safe adapter ID and environment variable names.
/// - `catalog`: Model descriptors and adapter-only request mappings.
/// - `fallback_model`: Model name used for provider configuration defaults.
///
/// A configured provider adapter for that connection.
///
/// Returns a configuration failure when the endpoint is missing or unsupported.
/// # Returns
/// The configured provider adapter.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn from_connection(
    connection: &catalog::ProviderConnection,
    catalog: &catalog::ProviderCatalog,
    fallback_model: &str,
) -> Result<NativeProvider, ProviderError> {
    if connection.provider != "openai_compatible" {
        return Err(ProviderError::Configuration(format!(
            "unsupported provider family for connection {}",
            connection.id
        )));
    }
    let base_url = std::env::var(&connection.base_url_env).map_err(|_| {
        ProviderError::Configuration(format!(
            "provider endpoint environment variable is unavailable for {}",
            connection.id
        ))
    })?;
    let credential = connection
        .api_key_env
        .as_ref()
        .map_or(CredentialSource::None, |name| {
            CredentialSource::Environment(name.clone())
        });
    let config = ProviderConfig::new(
        connection.id.clone(),
        base_url,
        fallback_model.to_owned(),
        credential,
    );
    let mut provider =
        scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider::new(config)?;
    for model in &catalog.library.models {
        let uses_connection = catalog
            .library
            .profiles
            .iter()
            .any(|profile| profile.model == model.id && profile.connection == connection.id);
        if !uses_connection {
            continue;
        }
        for (provider_model, level, parameters) in &catalog.thinking_parameters {
            if provider_model == &model.provider_model_id {
                provider = provider.with_model_thinking_parameters(
                    provider_model.clone(),
                    level.clone(),
                    parameters.clone(),
                );
            }
        }
    }
    Ok(NativeProvider::OpenAiCompatible(Box::new(provider)))
}

/// Selects a registered deterministic provider scenario.
///
/// Configured deterministic scenario.
///
/// Returns a configuration error when the scenario identifier is unknown.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
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
        "self_hosting_php_implementation_cancellation" => self_hosting(
            ValidationMode::Normal,
            Some(CancellationPhase::Implementation),
        ),
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
/// - `validation`: First-candidate validation behavior.
/// - `cancellation`: Optional delayed operation phase.
///
/// Provider-neutral deterministic PHP scenario.
/// # Returns
/// The operation result.
fn self_hosting(
    validation: ValidationMode,
    cancellation: Option<CancellationPhase>,
) -> NativeProvider {
    NativeProvider::SelfHostingPhp(crate::self_hosting_provider::PhpSelfHostingProvider {
        validation,
        cancellation,
    })
}
