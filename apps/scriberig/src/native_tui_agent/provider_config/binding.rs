//! Builds a provider binding for one catalog-resolved model selection.

use std::collections::BTreeMap;

use rpi_ai::{Model, Provider, RequestSecret};
use scriberig_agent_runtime::host_configuration::HostConfigurationError;

use crate::agent_config::AuthorizedCredentialProvider;
use crate::native_scenario::NativeScenario;
use crate::native_tui_agent::NativeTuiAgentRunner;

type ProviderBinding = Option<(std::sync::Arc<dyn Provider>, Model, Option<RequestSecret>)>;
type ProviderBindingResult = Result<ProviderBinding, HostConfigurationError>;

/// Creates a provider only from host-owned connections or upstream built-ins.
///
/// # Parameters
/// - `providers`: Trusted in-process providers keyed by safe connection id.
/// - `extensions`: Published upstream provider-extension runtime.
/// - `credentials`: Authorized host credential adapter.
/// - `upstream_enabled`: Whether upstream built-in providers are enabled.
/// - `model`: Exact model resolved from the upstream catalog snapshot.
/// - `connection`: Safe connection identity selected with the model.
///
/// # Returns
/// The provider, exact model, and authorized request secret when available.
///
/// # Errors
/// Returns a safe host configuration error for unsupported provider settings.
pub(in crate::native_tui_agent) fn provider_for(
    providers: &BTreeMap<String, NativeScenario>,
    extensions: &scriberig_app::ProviderExtensionRuntime,
    credentials: Option<&AuthorizedCredentialProvider>,
    upstream_enabled: bool,
    model: &Model,
    connection: &str,
) -> ProviderBindingResult {
    if let Some(provider) = providers.get(connection) {
        return Ok(Some((
            provider.clone().into_provider(model.clone()),
            model.clone(),
            None,
        )));
    }
    if let Some(provider) = extensions.provider(&model.provider) {
        let request_secret = authorized_secret(credentials, model)?;
        return Ok(Some((provider, model.clone(), request_secret)));
    }
    if !upstream_enabled {
        return Ok(None);
    }
    let provider = rpi_model_runtime::create_builtin_provider(
        &model.provider,
        vec![model.clone()],
        reqwest::Client::new(),
    )
    .map_err(|error| HostConfigurationError::Configuration(error.to_string()))?;
    let request_secret = authorized_secret(credentials, model)?;
    Ok(Some((provider, model.clone(), request_secret)))
}

/// Resolves a host-authorized request credential for one exact model.
///
/// # Parameters
/// - `credentials`: Optional authorized host credential adapter.
/// - `model`: Exact catalog model that will receive the credential.
///
/// # Returns
/// The protected request secret when the host adapter authorizes one.
///
/// # Errors
/// Returns a safe host configuration error when credential authorization fails.
fn authorized_secret(
    credentials: Option<&AuthorizedCredentialProvider>,
    model: &Model,
) -> Result<Option<RequestSecret>, HostConfigurationError> {
    let connection_credentials =
        crate::agent_config::authorized_connection_credential_provider(model)
            .map_err(HostConfigurationError::Configuration)?;
    let credentials = connection_credentials.as_ref().or(credentials);
    credentials
        .map(AsRef::as_ref)
        .map(|credentials| {
            scriberig_agent_runtime::protected_credential::request_secret_for_model(
                credentials,
                model,
            )
        })
        .transpose()
        .map_err(|error| HostConfigurationError::Configuration(error.to_string()))
        .map(Option::flatten)
}

impl NativeTuiAgentRunner {
    /// Creates the selected provider binding through the shared host factory.
    ///
    /// # Parameters
    /// - `model`: Exact model retained from the resolved catalog snapshot.
    /// - `connection`: Host-owned provider connection selected with the model.
    ///
    /// # Returns
    /// The provider, exact model, and protected request secret when supported.
    ///
    /// # Errors
    /// Returns a safe configuration or credential authorization failure.
    pub(in crate::native_tui_agent) fn upstream_provider_for(
        &self,
        model: &Model,
        connection: &str,
    ) -> ProviderBindingResult {
        provider_for(
            &self.providers,
            &self.provider_extension_runtime,
            self.upstream_credentials.as_ref(),
            self.upstream_enabled,
            model,
            connection,
        )
    }
}
