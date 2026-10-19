//! Resolves layered Scriberig settings and reports their origins.

use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use super::credentials;
use serde_json::Value;

use super::files::{
    find_config_file, project_config_dir, read_config_file, system_config_dir, user_config_dir,
};

/// Identifies the source that supplied one effective setting.
///
/// # Public members
/// - `File`: The path to a configuration file.
/// - `Environment`: The environment variable name.
/// - `CommandLine`: The explicit CLI option name.
/// - `Default`: The built-in default marker.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigSource {
    /// Configuration file that supplied the property.
    File(PathBuf),
    /// Environment variable that supplied the property.
    Environment(String),
    /// Explicit command-line option that supplied the property.
    CommandLine(String),
    /// Built-in Scriberig default.
    Default,
}

/// Stores an effective property value and its origin.
///
/// # Public members
/// - `value`: Effective JSON-compatible value.
/// - `source`: Winning configuration source.
///
/// # Private members
/// None.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigEntry {
    /// Effective value in JSON-compatible form.
    pub value: Value,
    /// Source that supplied the effective value.
    pub source: ConfigSource,
}

/// Holds resolved settings in stable property-name order.
///
/// # Public members
/// - `entries`: Effective properties and sources.
///
/// # Private members
/// None.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedConfig {
    /// Resolved properties.
    pub entries: BTreeMap<String, ConfigEntry>,
}

/// Resolves settings from files, environment variables, and CLI overrides.
///
/// # Public members
/// - `from_process`: Creates a resolver from process settings.
/// - `with_inputs`: Creates a resolver from explicit test inputs.
/// - `resolve`: Computes effective property values and sources.
///
/// # Private members
/// - `system_dir`: Optional system configuration directory.
/// - `user_dir`: Optional user configuration directory.
/// - `current_dir`: Project discovery starting directory.
/// - `environment`: Environment variables used in resolution.
pub struct ConfigResolver {
    /// Optional system configuration directory.
    system_dir: Option<PathBuf>,
    /// Optional user configuration directory.
    user_dir: Option<PathBuf>,
    /// Project discovery starting directory.
    current_dir: PathBuf,
    /// Environment variables used in resolution.
    environment: BTreeMap<String, String>,
}

impl ConfigResolver {
    /// Creates a resolver using the current process environment and directory.
    ///
    /// A resolver using conventional platform configuration locations.
    ///
    /// Returns an error when the current directory cannot be read.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn from_process() -> Result<Self, std::io::Error> {
        Ok(Self {
            system_dir: system_config_dir(),
            user_dir: user_config_dir(),
            current_dir: env::current_dir()?,
            environment: env::vars()
                .filter(|(name, _)| name.starts_with("SRIG_"))
                .collect(),
        })
    }

    /// Creates a resolver with deterministic paths and environment values.
    ///
    /// - `system_dir`: Optional system configuration directory.
    /// - `user_dir`: Optional user configuration directory.
    /// - `current_dir`: Starting directory for project-file discovery.
    /// - `environment`: Environment variables visible to resolution.
    ///
    /// A resolver with the supplied inputs.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn with_inputs(
        system_dir: Option<PathBuf>,
        user_dir: Option<PathBuf>,
        current_dir: PathBuf,
        environment: BTreeMap<String, String>,
    ) -> Self {
        Self {
            system_dir,
            user_dir,
            current_dir,
            environment,
        }
    }

    /// Resolves known properties with independent per-property precedence.
    ///
    /// - `command_line`: Explicit CLI values keyed by property name.
    ///
    /// Effective properties and their winning sources.
    ///
    /// Returns an error for malformed files, unknown properties, or bad types.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn resolve(
        &self,
        command_line: &BTreeMap<String, Value>,
    ) -> Result<ResolvedConfig, ConfigError> {
        let mut entries = defaults();
        let project_dir = project_config_dir(&self.current_dir);

        // Read each file level in ascending precedence order.
        for directory in [
            self.system_dir.as_deref(),
            self.user_dir.as_deref(),
            project_dir.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(path) = find_config_file(directory) {
                let layer = read_config_file(&path)?;
                credentials::validate_aliases(layer.iter().map(|(name, _)| name.as_str()))?;
                for (name, value) in layer {
                    let (name, value) = credentials::normalize(name, value)?;
                    set_entry(&mut entries, name, value, ConfigSource::File(path.clone()))?;
                }
            }
        }

        // Apply environment values property by property.
        credentials::validate_aliases(
            properties()
                .iter()
                .copied()
                .filter(|property| self.environment.contains_key(&environment_name(property))),
        )?;
        for property in properties().iter().copied() {
            let variable = environment_name(property);
            if let Some(value) = self.environment.get(&variable) {
                let value = parse_environment_value(property, value)?;
                let (name, value) = credentials::normalize(property.to_owned(), value)?;
                set_entry(
                    &mut entries,
                    name,
                    value,
                    ConfigSource::Environment(variable),
                )?;
            }
        }

        // Explicit CLI values override every other source.
        credentials::validate_aliases(command_line.keys().map(String::as_str))?;
        for (name, value) in command_line {
            let (canonical_name, value) = credentials::normalize(name.clone(), value.clone())?;
            set_entry(
                &mut entries,
                canonical_name,
                value,
                ConfigSource::CommandLine(if name == "credential" {
                    "--credential-ref".to_owned()
                } else {
                    format!("--{}", name.replace('_', "-"))
                }),
            )?;
        }
        Ok(ResolvedConfig { entries })
    }
}

/// Describes a configuration loading or validation error.
///
/// # Public members
/// - `message`: Safe human-readable diagnostic.
///
/// # Private members
/// None.
#[derive(Debug)]
pub struct ConfigError {
    /// Safe diagnostic message.
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    /// Formats the configuration error.
    ///
    /// - `self`: Error to format.
    /// - `formatter`: Output formatter.
    ///
    /// Formatting success or the formatter error.
    ///
    /// Returns the formatter error when writing fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ConfigError {}

/// Returns the supported configuration property names.
///
/// Supported property names in stable order.
/// # Returns
/// The operation result.
pub(super) fn properties() -> &'static [&'static str] {
    &[
        "api_key_env",
        "credential",
        "base_url",
        "db",
        "format",
        "model",
        "no_stream",
        "provider",
    ]
}

/// Builds the initial effective values from Scriberig's command defaults.
///
/// Default values with `Default` provenance.
/// # Returns
/// The operation result.
fn defaults() -> BTreeMap<String, ConfigEntry> {
    [
        ("credential", Value::Null),
        ("base_url", Value::Null),
        (
            "db",
            Value::String("playground/dev-app-runtime/scriberig.sqlite".to_owned()),
        ),
        ("format", Value::String("human".to_owned())),
        ("model", Value::String("default".to_owned())),
        ("no_stream", Value::Bool(false)),
        ("provider", Value::String("default".to_owned())),
    ]
    .into_iter()
    .map(|(name, value)| {
        (
            name.to_owned(),
            ConfigEntry {
                value,
                source: ConfigSource::Default,
            },
        )
    })
    .collect()
}

/// Inserts one value after validating its property type.
///
/// - `entries`: Resolved property map to update.
/// - `name`: Property name.
/// - `value`: Candidate property value.
/// - `source`: Origin of the candidate value.
///
/// Unit when the value is valid.
///
/// Returns an error when the property or value type is unsupported.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn set_entry(
    entries: &mut BTreeMap<String, ConfigEntry>,
    name: String,
    value: Value,
    source: ConfigSource,
) -> Result<(), ConfigError> {
    let valid = match name.as_str() {
        "credential" => {
            credentials::validate(&value)?;
            true
        }
        "api_key_env" | "base_url" => value.is_null() || value.is_string(),
        "db" | "format" | "model" | "provider" => value.is_string(),
        "no_stream" => value.is_boolean(),
        _ => false,
    };
    if !valid {
        return Err(ConfigError {
            message: format!("invalid value type for configuration property `{name}`"),
        });
    }
    if name == "format" && !matches!(value.as_str(), Some("human" | "json")) {
        return Err(ConfigError {
            message: "configuration property `format` must be human or json".to_owned(),
        });
    }
    if name == "base_url" {
        if let Some(endpoint) = value.as_str() {
            scriberig_secrets::validate_public_endpoint(endpoint).map_err(|_| ConfigError {
                message: "base_url must be an HTTP(S) URL without embedded credentials, queries, or fragments".to_owned(),
            })?;
        }
    }
    entries.insert(name, ConfigEntry { value, source });
    Ok(())
}

/// Converts a property name into its environment-variable name.
///
/// - `property`: Configuration property name.
///
/// The uppercase `SRIG_` environment-variable name.
/// # Returns
/// The operation result.
fn environment_name(property: &str) -> String {
    format!("SRIG_{}", property.to_ascii_uppercase())
}

/// Parses an environment string using the property's expected scalar type.
/// Parses an environment value using the property's expected scalar type.
///
/// - `property`: Configuration property name.
/// - `value`: Raw environment text.
///
/// Parsed JSON-compatible scalar value.
///
/// Returns an error when a typed value cannot be parsed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn parse_environment_value(property: &str, value: &str) -> Result<Value, ConfigError> {
    match property {
        "credential" => serde_json::from_str(value).map_err(|_| ConfigError {
            message: "SRIG_CREDENTIAL must contain a JSON credential reference or null".to_owned(),
        }),
        "no_stream" => value
            .parse::<bool>()
            .map(Value::Bool)
            .map_err(|_| ConfigError {
                message: format!("{} must be true or false", environment_name(property)),
            }),
        "api_key_env" | "base_url" | "db" | "format" | "model" | "provider" => {
            Ok(Value::String(value.to_owned()))
        }
        _ => Err(ConfigError {
            message: format!("unknown configuration property `{property}`"),
        }),
    }
}
