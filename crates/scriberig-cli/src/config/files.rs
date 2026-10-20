//! Discovers and parses system, user, and project configuration files.

use std::env;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::resolver::{ConfigError, properties};

/// Reads one supported YAML or JSON object from disk.
///
/// - `path`: Discovered configuration path.
///
/// Validated property values from the file.
///
/// Returns an error when the file cannot be read or parsed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn read_config_file(path: &Path) -> Result<Vec<(String, Value)>, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|error| ConfigError {
        message: format!("could not read {}: {error}", path.display()),
    })?;
    let parsed = match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => serde_json::from_str::<Value>(&text).map_err(|error| error.to_string()),
        Some("yaml" | "yml") => serde_yaml::from_str::<Value>(&text)
            .map_err(|error| error.to_string())
            .and_then(|value| serde_json::to_value(value).map_err(|error| error.to_string())),
        _ => unreachable!("unsupported config extension"),
    }
    .map_err(|_| ConfigError {
        message: format!("invalid configuration file {}", path.display()),
    })?;
    let object = parsed.as_object().ok_or_else(|| ConfigError {
        message: format!(
            "configuration file {} must contain an object",
            path.display()
        ),
    })?;
    object
        .iter()
        .map(|(name, value)| {
            if !properties().contains(&name.as_str()) {
                return Err(ConfigError {
                    message: format!("unknown configuration property in {}", path.display()),
                });
            }
            Ok((name.clone(), value.clone()))
        })
        .collect()
}

/// Finds the first supported config filename in a directory.
///
/// - `directory`: Configuration directory to inspect.
///
/// The selected existing config path, when present.
/// # Returns
/// The operation result.
pub(super) fn find_config_file(directory: &Path) -> Option<PathBuf> {
    ["config.yaml", "config.yml", "config.json"]
        .into_iter()
        .map(|name| directory.join(name))
        .find(|path| path.is_file())
}

/// Finds the nearest ancestor with a supported project configuration file.
///
/// - `current_dir`: Directory where project discovery starts.
///
/// The directory containing the nearest project configuration.
/// # Returns
/// The operation result.
pub(super) fn project_config_dir(current_dir: &Path) -> Option<PathBuf> {
    current_dir
        .ancestors()
        .find(|directory| find_config_file(directory).is_some())
        .map(Path::to_path_buf)
}

/// Returns the conventional platform system configuration directory.
///
/// The platform system config directory, when one is available.
/// # Returns
/// The operation result.
#[allow(clippy::unnecessary_wraps)]
pub(super) fn system_config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("PROGRAMDATA").map(|path| PathBuf::from(path).join("Scriberig"))
    }
    #[cfg(not(windows))]
    {
        Some(PathBuf::from("/etc/scriberig"))
    }
}

/// Returns the conventional platform user configuration directory.
///
/// The platform user config directory, when one is available.
/// # Returns
/// The operation result.
pub(super) fn user_config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("APPDATA").map(|path| PathBuf::from(path).join("Scriberig"))
    }
    #[cfg(target_os = "macos")]
    {
        env::var_os("HOME")
            .map(|path| PathBuf::from(path).join("Library/Application Support/Scriberig"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|path| PathBuf::from(path).join(".config")))
            .map(|path| path.join("scriberig"))
    }
}
