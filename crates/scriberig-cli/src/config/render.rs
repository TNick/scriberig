//! Renders effective configuration values with their origins.

use super::{ConfigError, ConfigSource, ResolvedConfig};

/// Renders every resolved property and its source designator.
///
/// - `config`: Effective configuration to render.
/// - `json`: Whether to produce JSON output.
///
/// Human-readable or JSON configuration output.
///
/// Returns an error when JSON serialization fails.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn render(config: &ResolvedConfig, json: bool) -> Result<String, ConfigError> {
    let rows = config
        .entries
        .iter()
        .map(|(property, entry)| {
            serde_json::json!({
                "property": property,
                "value": entry.value.clone(),
                "source": source_designator(&entry.source),
            })
        })
        .collect::<Vec<_>>();
    if json {
        return serde_json::to_string_pretty(&rows).map_err(|error| ConfigError {
            message: format!("could not render configuration: {error}"),
        });
    }
    Ok(rows
        .iter()
        .map(|row| {
            format!(
                "{} = {} ({})",
                row["property"].as_str().unwrap_or_default(),
                row["value"],
                row["source"].as_str().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Converts a configuration source to a stable user-facing designator.
///
/// - `source`: Winning source for one setting.
///
/// A file path, environment name, CLI option, or default label.
#[must_use]
/// # Returns
/// The operation result.
pub fn source_designator(source: &ConfigSource) -> String {
    match source {
        ConfigSource::File(path) => path.display().to_string(),
        ConfigSource::Environment(name) => format!("env:{name}"),
        ConfigSource::CommandLine(name) => format!("cli:{name}"),
        ConfigSource::Default => "default".to_owned(),
    }
}
