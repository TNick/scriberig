//! Exposes and persists provider-declared properties from upstream model rows.

use rpi_model_runtime::ModelsConfig;
use scriberig_agent_runtime::host_configuration::HostConfigurationError;

use super::NativeTuiAgentRunner;

impl NativeTuiAgentRunner {
    /// Reads upstream descriptors for a persisted provider model.
    ///
    /// # Parameters
    /// - `provider_id`: Exact upstream provider identity.
    /// - `model_id`: Provider-native model identity.
    ///
    /// # Returns
    /// Safe upstream property descriptors for an editable config row.
    ///
    /// # Errors
    /// Returns an error when the row is read-only or has an invalid schema.
    pub(super) fn model_properties(
        &self,
        provider_id: &str,
        model_id: &str,
    ) -> Result<Vec<rpi_model_runtime::ModelPropertyDescriptor>, HostConfigurationError> {
        {
            let configuration = self.model_config.lock().map_err(|_| {
                HostConfigurationError::Configuration(
                    "model configuration is unavailable".to_owned(),
                )
            })?;
            require_editable_row(
                &configuration.0,
                configuration.1.is_some(),
                provider_id,
                model_id,
            )?;
        }
        let model = self
            .model_catalog
            .snapshot()
            .models
            .into_iter()
            .find(|model| model.provider == provider_id && model.id == model_id)
            .ok_or_else(|| {
                HostConfigurationError::Configuration("configured model is unavailable".to_owned())
            })?;
        rpi_model_runtime::model_property_descriptors(&model).map_err(|_| {
            HostConfigurationError::Configuration(
                "upstream model property schema is invalid".to_owned(),
            )
        })
    }

    /// Validates, persists, and publishes one upstream model property edit.
    ///
    /// # Parameters
    /// - `provider_id`: Exact upstream provider identity.
    /// - `model_id`: Provider-native model identity.
    /// - `key`: Provider-declared property key.
    /// - `value`: User value passed to upstream validation.
    ///
    /// # Errors
    /// Returns an error when the row is read-only or upstream rejects the edit.
    ///
    /// # Returns
    /// `Ok(())` after the validated property is persisted.
    pub(super) fn set_model_property(
        &self,
        provider_id: &str,
        model_id: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), HostConfigurationError> {
        let model = self
            .model_catalog
            .snapshot()
            .models
            .into_iter()
            .find(|model| model.provider == provider_id && model.id == model_id)
            .ok_or_else(|| {
                HostConfigurationError::Configuration("configured model is unavailable".to_owned())
            })?;
        let safe_property = rpi_model_runtime::model_property_descriptors(&model)
            .map_err(|_| {
                HostConfigurationError::Configuration(
                    "upstream model property schema is invalid".to_owned(),
                )
            })?
            .into_iter()
            .find(|property| property.key == key)
            .is_some_and(|property| property.is_safe_for_generic_editor());
        if !safe_property {
            return Err(HostConfigurationError::Configuration(
                "model property is not available in the generic editor".to_owned(),
            ));
        }

        let mut configuration = self.model_config.lock().map_err(|_| {
            HostConfigurationError::Configuration("model configuration is unavailable".to_owned())
        })?;
        let (config, path) = &mut *configuration;
        require_editable_row(config, path.is_some(), provider_id, model_id)?;
        let path = path.as_deref().ok_or_else(read_only_configuration)?;

        let mut updated_config = config.clone();
        rpi_model_runtime::update_model_config_property(
            &mut updated_config,
            provider_id,
            model_id,
            key,
            value,
        )
        .map_err(|_| {
            HostConfigurationError::Configuration(
                "upstream model configuration update failed".to_owned(),
            )
        })?;
        let endpoint_config = updated_config.clone();
        let previous_models = self.model_catalog.snapshot().models;
        let resolver = super::provider_config::catalog::EnvironmentResolver;
        self.model_catalog.refresh_from_configuration_with_commit(
            &mut updated_config,
            &[],
            &resolver,
            |models| {
                rpi_model_runtime::resolve_model_endpoints(models, &endpoint_config, &resolver)
                    .map_err(|_| {
                        HostConfigurationError::Configuration(
                            "configured model endpoint could not be resolved".to_owned(),
                        )
                    })?;
                preserve_host_endpoints(models, &endpoint_config, &previous_models);
                for model in models {
                    model.headers = None;
                }
                Ok(())
            },
            |candidate| {
                rpi_model_runtime::ModelsConfigFile::save(path, candidate).map_err(|_| {
                    HostConfigurationError::Configuration(
                        "upstream model configuration persistence failed".to_owned(),
                    )
                })
            },
        )?;
        *config = updated_config;
        Ok(())
    }
}

/// Retains resolved host endpoint overrides when config has no endpoint value.
///
/// The fallback endpoint override is process configuration rather than a
/// provider model property, so a property edit must not reset it while
/// rebuilding the upstream catalog.
///
/// # Parameters
/// - `models`: Candidate upstream catalog rows being refreshed.
/// - `config`: Candidate upstream model config.
/// - `previous_models`: Previously published rows with resolved host endpoints.
fn preserve_host_endpoints(
    models: &mut [rpi_ai::Model],
    config: &ModelsConfig,
    previous_models: &[rpi_ai::Model],
) {
    for model in models {
        let configured_endpoint = config.providers.get(&model.provider).and_then(|provider| {
            provider
                .models
                .iter()
                .find(|definition| definition.id == model.id)
                .and_then(|definition| definition.base_url.as_deref())
                .or(provider.base_url.as_deref())
        });
        if configured_endpoint.is_some() {
            continue;
        }
        if let Some(previous) = previous_models
            .iter()
            .find(|previous| previous.provider == model.provider && previous.id == model.id)
        {
            model.base_url.clone_from(&previous.base_url);
        }
    }
}

/// Requires a file-backed config row before displaying or saving properties.
///
/// # Parameters
/// - `config`: Upstream configuration selected by the host.
/// - `has_persistent_path`: Whether the host selected a writable config file.
/// - `provider_id`: Exact upstream provider identity.
/// - `model_id`: Provider-native model identity.
///
/// # Errors
/// Returns a generic read-only error when this source cannot be edited.
///
/// # Returns
/// `Ok(())` when the model row is editable.
fn require_editable_row(
    config: &ModelsConfig,
    has_persistent_path: bool,
    provider_id: &str,
    model_id: &str,
) -> Result<(), HostConfigurationError> {
    let has_row = config
        .providers
        .get(provider_id)
        .is_some_and(|provider| provider.models.iter().any(|model| model.id == model_id));
    if has_persistent_path && has_row {
        Ok(())
    } else {
        Err(read_only_configuration())
    }
}

/// Creates a safe error for sources that are not file-backed config rows.
///
/// # Returns
/// A generic provider configuration error.
fn read_only_configuration() -> HostConfigurationError {
    HostConfigurationError::Configuration("model properties are read-only".to_owned())
}
