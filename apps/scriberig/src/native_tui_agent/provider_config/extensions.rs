//! Rejects native extensions until execution has a capability-scoped process boundary.

use scriberig_agent_runtime::host_configuration::HostConfigurationError;

/// Rejects extension paths that would load third-party native code in-process.
///
/// # Returns
/// `Ok(())` when no extension paths are configured.
///
/// # Errors
/// Returns a safe configuration error when an extension root is configured.
pub(crate) fn reject_in_process_extensions() -> Result<(), HostConfigurationError> {
    let configured = ["SCRIBE_AGENT_EXTENSION_DIRS", "SCRIBE_AGENT_PACKAGE_ROOTS"]
        .into_iter()
        .any(path_list_is_nonempty);
    if configured {
        return Err(HostConfigurationError::Configuration(
            "native provider extensions are disabled until upstream provides isolated worker execution"
                .to_owned(),
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
