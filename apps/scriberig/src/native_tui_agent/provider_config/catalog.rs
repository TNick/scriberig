//! Loads upstream provider models and creates a safe protocol display view.

use std::collections::BTreeMap;

use rpi_ai::{Api, Model};
use rpi_model_runtime::{ConfigValueResolver, ModelsConfig, build_catalog};
use scriberig_agent_runtime::provider::ProviderError;
/// Holds authoritative upstream models and native adapter connections.
///
/// # Public members
/// - Internal fields expose the upstream catalog and adapters to native
///   composition.
///
/// # Private members
/// None.
pub(in crate::native_tui_agent) struct ProviderCatalog {
    /// Upstream models used for model resolution and request configuration.
    pub(in crate::native_tui_agent) models: Vec<Model>,
    /// Credential-free provider connections supported by the native adapter.
    pub(in crate::native_tui_agent) connections: Vec<ProviderConnection>,
}

/// Describes one catalog provider adapter without credential material.
///
/// # Public members
/// - Fields identify the provider adapter and its endpoint.
///
/// # Private members
/// None.
pub(in crate::native_tui_agent) struct ProviderConnection {
    /// Provider-qualified model identity used by this adapter.
    pub(in crate::native_tui_agent) id: String,
    /// Adapter family supported by the native runtime.
    pub(in crate::native_tui_agent) provider: String,
    /// Resolved non-secret endpoint URL.
    pub(in crate::native_tui_agent) base_url: String,
}

/// Resolves literal values and environment references without running commands.
struct EnvironmentResolver;

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
        if let Some(name) = value
            .strip_prefix("${")
            .and_then(|value| value.strip_suffix('}'))
        {
            return environment
                .and_then(|values| values.get(name).cloned())
                .or_else(|| std::env::var(name).ok());
        }
        if let Some(name) = value.strip_prefix('$') {
            return environment
                .and_then(|values| values.get(name).cloned())
                .or_else(|| std::env::var(name).ok());
        }
        Some(value.to_owned())
    }

    /// Resolves values without using cached host state.
    ///
    /// # Parameters
    /// - `value`: Literal or environment variable reference.
    /// - `environment`: Optional scoped environment overlay.
    ///
    /// # Returns
    /// Resolved value, or `None` when the variable is unavailable.
    fn resolve_uncached(
        &self,
        value: &str,
        environment: Option<&BTreeMap<String, String>>,
    ) -> Option<String> {
        self.resolve(value, environment)
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
fn strip_unsupported_catalog_auth(config: &mut ModelsConfig) -> Result<(), ProviderError> {
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
        Ok(())
    }
}

/// Loads upstream models from `SCRIBE_AGENT_MODEL_CATALOG`.
///
/// The variable contains upstream `ModelsConfig` JSON. Only `OpenAI` Completions
/// entries can currently be invoked by Scriberig's native provider adapter.
///
/// - `fallback_model`: Model used when no provider configuration is present.
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
) -> Result<ProviderCatalog, ProviderError> {
    let json = std::env::var("SCRIBE_AGENT_MODEL_CATALOG").ok();
    let mut config = match json.as_deref() {
        Some(json) => {
            let value =
                serde_json::from_str::<serde_json::Value>(json).map_err(|_| invalid_catalog())?;
            if !value
                .as_object()
                .is_some_and(|object| object.contains_key("providers"))
            {
                return Err(invalid_catalog());
            }
            serde_json::from_value::<ModelsConfig>(value).map_err(|_| invalid_catalog())?
        }
        None => ModelsConfig::default(),
    };
    insert_fallback_provider(&mut config, fallback_model);
    strip_unsupported_catalog_auth(&mut config)?;

    let resolver = EnvironmentResolver;
    let mut models = build_catalog(&mut config, &[], &resolver);
    models.retain(|model| model.api == Api::OpenaiCompletions);
    if models.is_empty() {
        return Err(invalid_catalog());
    }
    for model in &mut models {
        model.headers = None;
    }
    let connections = models
        .iter()
        .map(|model| {
            let provider = config
                .providers
                .get(&model.provider)
                .ok_or_else(invalid_catalog)?;
            let model_config = provider.models.iter().find(|item| item.id == model.id);
            let base_url = model_config
                .and_then(|item| item.base_url.as_deref())
                .or(provider.base_url.as_deref())
                .and_then(|value| resolver.resolve(value, None))
                .unwrap_or_else(|| model.base_url.clone());
            Ok(ProviderConnection {
                id: connection_identity(&model.provider, &model.id),
                provider: "openai_compatible".to_owned(),
                base_url,
            })
        })
        .collect::<Result<Vec<_>, ProviderError>>()?;
    Ok(ProviderCatalog {
        models,
        connections,
    })
}

/// Creates the same generic failure for invalid catalog data.
///
/// # Returns
/// A configuration error that contains no provider values.
fn invalid_catalog() -> ProviderError {
    ProviderError::Configuration("invalid upstream model catalog".to_owned())
}

/// Rejects auth configuration that cannot use Scriberig's authorized host path.
///
/// # Returns
/// A configuration error with no credential or header values.
fn unsupported_catalog_auth_error() -> ProviderError {
    ProviderError::Configuration(
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
/// A versioned, byte-length-prefixed identity for projections and adapter lookup.
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
