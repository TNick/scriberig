//! Validates isolated worker configuration before the host can use extensions.

use scriberig_agent_runtime::host_configuration::HostConfigurationError;
#[cfg(feature = "isolated-plugin-worker")]
use scriberig_agent_runtime::plugin_worker::PluginWorkerConfiguration;

/// Validates isolated plugin configuration and rejects unsupported package roots.
///
/// Worker configuration is checked through the upstream adapter, but the
/// worker is not yet composed into branch tool catalogs. Configured plugins
/// therefore remain disabled until that production boundary exists.
///
/// # Returns
/// `Ok(())` when no plugin or package paths are configured.
///
/// # Errors
/// Returns a safe configuration error for invalid or uncomposed extensions.
pub(crate) fn validate_isolated_plugin_configuration() -> Result<(), HostConfigurationError> {
    if path_list_is_nonempty("SCRIBE_AGENT_PACKAGE_ROOTS") {
        return Err(HostConfigurationError::Configuration(
            "package roots are unsupported until upstream package lifecycle is integrated"
                .to_owned(),
        ));
    }
    #[cfg(feature = "isolated-plugin-worker")]
    if PluginWorkerConfiguration::from_environment()
        .map_err(|error| {
            HostConfigurationError::Configuration(format!(
                "invalid isolated plugin config: {error}"
            ))
        })?
        .is_some()
    {
        return Err(HostConfigurationError::Configuration(
            "isolated plugin workers are not yet registered in the branch tool catalog".to_owned(),
        ));
    }
    #[cfg(not(feature = "isolated-plugin-worker"))]
    if ["SCRIBE_AGENT_EXTENSION_DIRS", "SCRIBE_AGENT_PLUGIN_FILES"]
        .into_iter()
        .any(path_list_is_nonempty)
    {
        return Err(HostConfigurationError::Configuration(
            "isolated plugin support is not enabled in this build".to_owned(),
        ));
    }
    Ok(())
}

/// Checks whether an environment path list contains a non-empty path.
///
/// # Parameters
/// - `name`: Environment variable containing configured native-code paths.
///
/// # Returns
/// `true` when the process requested any extension path.
fn path_list_is_nonempty(name: &str) -> bool {
    std::env::var_os(name)
        .is_some_and(|value| std::env::split_paths(&value).any(|path| !path.as_os_str().is_empty()))
}
