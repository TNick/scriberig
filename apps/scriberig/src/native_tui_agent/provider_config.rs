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
/// - `shared_config`: Provider resolved from the shared application config.
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
    shared_config: Option<ProviderConfig>,
) -> Result<Option<NativeProvider>, ProviderError> {
    match provider_kind {
        "deterministic" => deterministic_provider(),
        "openai_compatible" => {
            let config = configured_provider_config(
                std::env::var("SCRIBE_AGENT_BASE_URL").ok(),
                std::env::var("SCRIBE_AGENT_API_KEY_ENV").ok(),
                model,
                shared_config,
            );
            config
                .map(|config| openai_compatible(config, catalog))
                .transpose()
        }
        unknown => Err(ProviderError::Configuration(format!(
            "unsupported SCRIBE_AGENT_PROVIDER: {unknown}"
        ))),
    }
}

/// Selects the legacy TUI override or shared application provider settings.
///
/// - `legacy_base_url`: Optional explicit TUI endpoint override.
/// - `legacy_credential`: Optional environment variable name for that endpoint.
/// - `model`: Effective model selected for the TUI request.
/// - `shared_config`: Provider settings resolved by the shared configuration.
///
/// Provider settings for one TUI request, without resolving credential values.
/// # Returns
/// The selected safe provider configuration, when configured.
fn configured_provider_config(
    legacy_base_url: Option<String>,
    legacy_credential: Option<String>,
    model: &str,
    shared_config: Option<ProviderConfig>,
) -> Option<ProviderConfig> {
    legacy_base_url
        .map(|base_url| {
            let credential =
                legacy_credential.map_or(CredentialSource::None, CredentialSource::Environment);
            ProviderConfig::new("local", base_url, model.to_owned(), credential)
        })
        .or_else(|| {
            shared_config.map(|mut config| {
                model.clone_into(&mut config.model);
                config
            })
        })
}

/// Builds an OpenAI-compatible adapter with catalog-specific thinking mappings.
///
/// - `config`: Effective endpoint, model, and indirect credential reference.
/// - `catalog`: Safe descriptors and provider-private thinking mappings.
///
/// The configured native provider adapter.
///
/// Returns an error when the provider configuration is invalid.
/// # Returns
/// The configured provider.
/// # Errors
/// Returns an error when the operation fails.
fn openai_compatible(
    config: ProviderConfig,
    catalog: &catalog::ProviderCatalog,
) -> Result<NativeProvider, ProviderError> {
    let mut provider =
        scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider::new(config)?;
    for (model, level, parameters) in &catalog.thinking_parameters {
        provider = provider.with_model_thinking_parameters(
            model.clone(),
            level.clone(),
            parameters.clone(),
        );
    }
    Ok(NativeProvider::OpenAiCompatible(Box::new(provider)))
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
        "arithmetic" => {
            NativeProvider::Arithmetic(crate::deterministic_provider::ArithmeticProvider)
        }
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

#[cfg(test)]
mod tests {
    use super::configured_provider_config;
    use scriberig_agent_runtime::provider::{CredentialSource, ProviderConfig};

    /// Uses shared doctor configuration when no TUI-specific endpoint is set.
    #[test]
    fn shared_provider_configuration_is_used_by_tui() {
        let shared = ProviderConfig::new(
            "llm-strata",
            "http://localhost:1234/v1",
            "configured-model",
            CredentialSource::Environment("API_TOKEN".to_owned()),
        );

        let selected = configured_provider_config(None, None, "configured-model", Some(shared))
            .expect("shared provider settings");

        assert_eq!(selected.profile_id, "llm-strata");
        assert_eq!(selected.base_url, "http://localhost:1234/v1");
        assert_eq!(selected.model, "configured-model");
        assert!(matches!(
            selected.credential_source,
            CredentialSource::Environment(ref name) if name == "API_TOKEN"
        ));
    }

    /// Keeps an explicit TUI endpoint ahead of shared provider configuration.
    #[test]
    fn explicit_tui_provider_configuration_remains_an_override() {
        let selected = configured_provider_config(
            Some("http://localhost:4321/v1".to_owned()),
            Some("TUI_TOKEN".to_owned()),
            "tui-model",
            Some(ProviderConfig::new(
                "shared",
                "http://localhost:1234/v1",
                "shared-model",
                CredentialSource::None,
            )),
        )
        .expect("TUI override");

        assert_eq!(selected.profile_id, "local");
        assert_eq!(selected.base_url, "http://localhost:4321/v1");
        assert_eq!(selected.model, "tui-model");
        assert!(matches!(
            selected.credential_source,
            CredentialSource::Environment(ref name) if name == "TUI_TOKEN"
        ));
    }
}
