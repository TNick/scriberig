//! Loads upstream provider models and creates a safe protocol display view.

use std::collections::BTreeMap;

use rpi_ai::Model;
use rpi_model_runtime::{ConfigValueResolver, ModelsConfig};
use scriberig_agent_runtime::provider::HostConfigurationError;
/// Holds authoritative upstream models from the configured model catalog.
///
/// # Public members
/// - `model_catalog`: Shared upstream catalog used by model resolution and providers.
///
/// # Private members
/// None.
pub(in crate::native_tui_agent) struct ProviderCatalog {
    /// Shared model catalog used by application projections and providers.
    pub(in crate::native_tui_agent) model_catalog: rpi_model_runtime::ModelCatalog,
    /// Sanitized upstream model configuration retained for host-side settings.
    pub(in crate::native_tui_agent) config: ModelsConfig,
    /// Exact host-selected path when the config came from a persisted file.
    pub(in crate::native_tui_agent) config_path: Option<std::path::PathBuf>,
}

/// Resolves literal values and environment references without running commands.
pub(in crate::native_tui_agent) struct EnvironmentResolver;

impl ConfigValueResolver for EnvironmentResolver {
    /// Resolves literals or `$NAME` and `${NAME}` expressions from the process.
    ///
    /// # Parameters
    /// - `value`: Literal or environment variable reference.
    /// - `environment`: Optional scoped environment overlay.
    ///
    /// # Returns
    /// Resolved value, or `None` when the variable is unavailable.
    fn resolve(
        &self,
        value: &str,
        environment: Option<&BTreeMap<String, String>>,
    ) -> Option<String> {
        rpi_model_runtime::resolve_environment_template(value, environment, |name| {
            std::env::var(name).ok()
        })
    }

    /// Reads one host environment variable.
    ///
    /// # Parameters
    /// - `name`: Environment variable name.
    ///
    /// # Returns
    /// The value when the variable is available.
    fn environment_value(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}

/// Adds the local fallback model when no provider is configured.
///
/// - `config`: Upstream catalog to update.
/// - `fallback_model`: Model ID used for the fallback entry.
fn insert_fallback_provider(config: &mut ModelsConfig, fallback_model: &str) {
    if !config.providers.is_empty() {
        return;
    }
    config.providers.insert(
        "openai".to_owned(),
        rpi_model_runtime::ProviderConfig {
            name: Some("OpenAI".to_owned()),
            base_url: None,
            api_key: None,
            api: Some("openai-completions".to_owned()),
            headers: None,
            auth_header: None,
            models: vec![rpi_model_runtime::ModelDefinition {
                id: fallback_model.to_owned(),
                name: None,
                base_url: None,
                reasoning: None,
                thinking_level_map: None,
                context_window: None,
                max_tokens: None,
                cost: None,
                sampling_params: None,
                provider_options: BTreeMap::new(),
                provider_options_schema: None,
                input: None,
                headers: None,
                compat: None,
            }],
        },
    );
}

/// Removes typed credentials and rejects catalog-supplied auth material.
///
/// - `config`: Upstream catalog whose auth fields are inspected and cleared.
///
/// Returns an error when typed credentials or request headers are configured.
/// # Returns
/// `Ok(())` when the catalog contains no unsupported auth material.
/// # Errors
/// Returns an error when catalog auth is unsupported by the host boundary.
fn strip_unsupported_catalog_auth(config: &mut ModelsConfig) -> Result<(), HostConfigurationError> {
    let has_auth = config.providers.values().any(|provider| {
        provider.api_key.is_some()
            || provider.auth_header.is_some()
            || provider
                .headers
                .as_ref()
                .is_some_and(|headers| !headers.is_empty())
            || provider.models.iter().any(|model| {
                model
                    .headers
                    .as_ref()
                    .is_some_and(|headers| !headers.is_empty())
            })
    });
    for provider in config.providers.values_mut() {
        provider.api_key = None;
        provider.auth_header = None;
        provider.headers = None;
        for model in &mut provider.models {
            model.headers = None;
        }
    }
    if has_auth {
        Err(unsupported_catalog_auth_error())
    } else {
        Ok::<(), HostConfigurationError>(())
    }
}

/// Loads upstream models from `SCRIBE_AGENT_MODEL_CATALOG`.
///
/// The variable contains upstream `ModelsConfig` JSON. Models are retained
/// when the pinned upstream runtime has a built-in provider implementation.
///
/// - `fallback_model`: Model used when no provider configuration is present.
/// - `endpoint_override`: Optional explicit legacy endpoint override for the
///   configured fallback model.
///
/// Upstream model authority and a safe display projection.
///
/// Returns a configuration error for malformed or unsupported configuration.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when catalog construction or display projection fails.
pub(in crate::native_tui_agent) fn from_environment(
    fallback_model: &str,
    endpoint_override: Option<&str>,
) -> Result<ProviderCatalog, HostConfigurationError> {
    let json = std::env::var("SCRIBE_AGENT_MODEL_CATALOG").ok();
    let config_path =
        std::env::var_os("SCRIBE_AGENT_MODEL_CONFIG_PATH").map(std::path::PathBuf::from);
    let (mut config, use_fallback) = match (config_path.as_deref(), json.as_deref()) {
        (Some(path), _) => (
            rpi_model_runtime::ModelsConfigFile::load(path).map_err(|_| invalid_catalog())?,
            false,
        ),
        (None, Some(json)) => {
            let value =
                serde_json::from_str::<serde_json::Value>(json).map_err(|_| invalid_catalog())?;
            if !value
                .as_object()
                .is_some_and(|object| object.contains_key("providers"))
            {
                return Err(invalid_catalog());
            }
            (
                serde_json::from_value::<ModelsConfig>(value).map_err(|_| invalid_catalog())?,
                false,
            )
        }
        (None, None) => (ModelsConfig::default(), true),
    };
    if use_fallback {
        insert_fallback_provider(&mut config, fallback_model);
    }
    strip_unsupported_catalog_auth(&mut config)?;

    let resolver = EnvironmentResolver;
    let endpoint_config = config.clone();
    let model_catalog = rpi_model_runtime::ModelCatalog::new();
    model_catalog.refresh_from_configuration(&mut config, &[], &resolver, |models| {
        rpi_model_runtime::resolve_model_endpoints(models, &endpoint_config, &resolver)
            .map_err(|_| invalid_catalog())?;
        if let Some(endpoint) = endpoint_override {
            apply_endpoint_override(models, fallback_model, endpoint)?;
        }
        for model in models {
            model.headers = None;
        }
        Ok::<(), HostConfigurationError>(())
    })?;
    Ok(ProviderCatalog {
        model_catalog,
        config,
        config_path,
    })
}

/// Applies a host endpoint override to the configured fallback model.
///
/// # Parameters
/// - `config`: Upstream provider configuration loaded for this process.
/// - `fallback_model`: Provider-native model ID selected by fallback logic.
/// - `endpoint`: Explicit endpoint value supplied by host configuration.
/// - `models`: Mutable upstream models receiving the validated endpoint.
///
/// # Returns
/// `Ok(())` when the endpoint is applied to exactly one model.
///
/// # Errors
/// Returns a generic configuration error when the endpoint or model binding is
/// invalid.
fn apply_endpoint_override(
    models: &mut [Model],
    fallback_model: &str,
    endpoint: &str,
) -> Result<(), HostConfigurationError> {
    if scriberig_secrets::validate_public_endpoint(endpoint).is_err() {
        return Err(invalid_catalog());
    }
    let mut matching = models
        .iter_mut()
        .filter(|model| model.id.eq_ignore_ascii_case(fallback_model));
    let model = matching.next().ok_or_else(invalid_catalog)?;
    if matching.next().is_some() {
        return Err(invalid_catalog());
    }
    endpoint.clone_into(&mut model.base_url);
    Ok(())
}

/// Creates the same generic failure for invalid catalog data.
///
/// # Returns
/// A configuration error that contains no provider values.
fn invalid_catalog() -> HostConfigurationError {
    HostConfigurationError::Configuration("invalid upstream model catalog".to_owned())
}

/// Rejects auth configuration that cannot use Scriberig's authorized host path.
///
/// # Returns
/// A configuration error with no credential or header values.
fn unsupported_catalog_auth_error() -> HostConfigurationError {
    HostConfigurationError::Configuration(
        "catalog credentials and headers require Scriberig host configuration".to_owned(),
    )
}

/// Encodes provider and model identities without delimiter collisions.
///
/// # Parameters
/// - `provider_id`: Upstream provider identity.
/// - `model_id`: Upstream model identity.
///
/// # Returns
/// A versioned, byte-length-prefixed identity for provider/model lookup.
pub(in crate::native_tui_agent) fn connection_identity(
    provider_id: &str,
    model_id: &str,
) -> String {
    format!(
        "rpi1:{}:{}{}:{}",
        provider_id.len(),
        provider_id,
        model_id.len(),
        model_id
    )
}
