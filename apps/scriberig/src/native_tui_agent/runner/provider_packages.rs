//! Refreshes explicitly configured provider package generations.

use super::super::NativeTuiAgentRunner;
use scriberig_agent_runtime::host_configuration::HostConfigurationError;

impl NativeTuiAgentRunner {
    /// Refreshes explicitly configured providers without restarting the product.
    ///
    /// The upstream generation gate keeps existing provider calls bound to the
    /// worker and catalog generation they already acquired.
    ///
    /// # Returns
    /// `Ok(())` after the package and catalog refresh completes.
    ///
    /// # Errors
    /// Returns a configuration error if package loading or atomic publication fails.
    pub(super) fn refresh_provider_packages(&self) -> Result<(), HostConfigurationError> {
        let worker = super::super::provider_config::extensions::activate_provider_packages(
            &self.provider_extension_runtime,
            &self.project_store,
            &self.extension_component_mailbox,
        )?;
        *self.plugin_command_worker.lock().map_err(|_| {
            HostConfigurationError::Configuration(
                "plugin command worker state is unavailable".to_owned(),
            )
        })? = worker;
        Ok(())
    }
}
