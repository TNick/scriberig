//! Implements deny-by-default host operations for isolated provider packages.

use scriberig_agent_runtime::plugin_worker::PluginSemanticDispatcher;
use std::collections::BTreeMap;
use std::sync::RwLock;

/// Denies host operations except UI updates and authorized provider HTTP calls.
#[cfg(feature = "isolated-plugin-worker")]
pub(super) struct DenyPluginOperations {
    /// Routes component actions to the product's shared upstream mailbox.
    components: scriberig_agent_runtime::plugin_worker::PluginComponentDispatcher,
    /// Exact host-admitted models allowed to use the provider HTTP broker.
    provider_models: RwLock<BTreeMap<(String, String, String), rpi_ai::Model>>,
}

#[cfg(feature = "isolated-plugin-worker")]
impl DenyPluginOperations {
    /// Creates a deny-by-default dispatcher with one permitted UI route.
    ///
    /// # Parameters
    /// - `mailbox`: Session-scoped upstream component registry.
    ///
    /// # Returns
    /// A dispatcher that accepts isolated plugin component updates.
    pub(super) fn new(mailbox: rpi_extensions::ExtensionStatusMailbox) -> Self {
        Self {
            components: scriberig_agent_runtime::plugin_worker::PluginComponentDispatcher::new(
                mailbox,
            ),
            provider_models: RwLock::new(BTreeMap::new()),
        }
    }

    /// Publishes the exact provider models admitted for host-mediated requests.
    ///
    /// # Parameters
    /// - `models`: Host-validated provider models.
    pub(super) fn set_provider_models(&self, models: Vec<rpi_ai::Model>) {
        let mut admitted = self
            .provider_models
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *admitted = models
            .into_iter()
            .map(|model| {
                (
                    (
                        model.provider.clone(),
                        model.id.clone(),
                        model.api.as_str().to_owned(),
                    ),
                    model,
                )
            })
            .collect();
    }

    /// Executes a provider request only for an exact host-admitted model.
    ///
    /// # Parameters
    /// - `arguments`: Worker-supplied provider identity and prepared request.
    ///
    /// # Returns
    /// Host response and host-generated applied-settings evidence.
    ///
    /// # Errors
    /// Returns a denial for unknown models or any failed/unsafe HTTP request.
    fn dispatch_provider_http_request(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if arguments.as_object().is_none_or(|object| object.len() != 4) {
            return Err("provider HTTP request identity is invalid".to_owned());
        }
        let provider = required_string(arguments, "provider")?;
        let model_id = required_string(arguments, "model")?;
        let api = required_string(arguments, "api")?;
        let request = arguments
            .get("request")
            .ok_or_else(|| "provider HTTP request payload is missing".to_owned())?;
        let key = (provider.to_owned(), model_id.to_owned(), api.to_owned());
        let models = self
            .provider_models
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let model = models
            .get(&key)
            .ok_or_else(|| "provider HTTP request model is not admitted".to_owned())?;
        let request_json = match request {
            serde_json::Value::String(value) => value.clone(),
            value => serde_json::to_string(value)
                .map_err(|_| "provider HTTP request is invalid".to_owned())?,
        };
        let response =
            rpi_extensions::isolated::execute_provider_http_request(model, &request_json)?;
        serde_json::from_str(&response).map_err(|_| "provider HTTP response is invalid".to_owned())
    }
}

#[cfg(feature = "isolated-plugin-worker")]
impl PluginSemanticDispatcher for DenyPluginOperations {
    /// Dispatches one narrowly authorized provider-worker host operation.
    ///
    /// # Parameters
    /// - `operation`: Requested operation identifier.
    /// - `arguments`: Operation arguments.
    ///
    /// # Returns
    /// The operation result when its host policy permits it.
    ///
    /// # Errors
    /// Returns a denial for every operation outside the explicit routes.
    fn dispatch(
        &self,
        operation: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match operation {
            "ui_component" => self.components.dispatch(arguments),
            "provider_http_request" => self.dispatch_provider_http_request(arguments),
            _ => Err(format!(
                "provider-only worker cannot request host operation `{operation}`"
            )),
        }
    }
}

/// Reads a required string field from a worker request.
///
/// # Parameters
/// - `value`: Worker-supplied JSON object.
/// - `key`: Required field name.
///
/// # Returns
/// The borrowed string field.
///
/// # Errors
/// Returns a generic request error when the field is absent or not a string.
#[cfg(feature = "isolated-plugin-worker")]
fn required_string<'a>(value: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "provider HTTP request identity is invalid".to_owned())
}

#[cfg(all(test, feature = "isolated-plugin-worker"))]
mod tests {
    use super::DenyPluginOperations;
    use scriberig_agent_runtime::plugin_worker::PluginSemanticDispatcher;

    /// Prevents a worker from selecting a model outside its host generation.
    #[test]
    fn provider_http_dispatch_requires_an_exact_admitted_model() {
        let dispatcher = DenyPluginOperations::new(rpi_extensions::ExtensionStatusMailbox::new());
        let model = rpi_ai::Model::new(
            "admitted-model",
            "Admitted model",
            rpi_ai::Api::OpenaiCompletions,
            "admitted-provider",
            "https://api.openai.com/v1",
        );
        dispatcher.set_provider_models(vec![model]);
        let admitted_arguments = serde_json::json!({
            "provider": "admitted-provider",
            "model": "admitted-model",
            "api": "openai-completions",
            "request": {
                "schemaVersion": "rpi-provider-http-request-v1",
                "method": "GET",
                "url": "https://api.openai.com/v1/chat/completions",
                "headers": {},
                "body": {}
            }
        });
        let admitted_error = dispatcher
            .dispatch("provider_http_request", &admitted_arguments)
            .expect_err("unsupported methods must be rejected after model authorization");
        assert!(admitted_error.contains("version or method is unsupported"));

        let mut untrusted_arguments = admitted_arguments;
        untrusted_arguments["provider"] = serde_json::json!("untrusted-provider");
        let error = dispatcher
            .dispatch("provider_http_request", &untrusted_arguments)
            .expect_err("unknown provider must be denied");
        assert!(error.contains("model is not admitted"));
    }
}
