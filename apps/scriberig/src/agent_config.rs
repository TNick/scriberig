//! Resolves and displays local agent configuration.

/// Resolves an explicitly selected `ScribeRig` secret through host policy.
pub(crate) type AuthorizedCredentialProvider =
    std::sync::Arc<dyn scriberig_secrets::CredentialProvider>;

/// Exact upstream model, host credential adapter, and configured state.
pub(crate) type ResolvedProviderSetup = (rpi_ai::Model, Option<AuthorizedCredentialProvider>, bool);

/// Prints the effective local-agent settings and their sources.
///
/// - `args`: Output format selected for the inspection command.
///
/// Unit after the resolved configuration is written to stdout.
///
/// Returns an error when configuration loading or rendering fails.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn config_command(
    args: &scriberig_cli::ConfigArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let scriberig_cli::ConfigCommand::Show(show) = &args.command;
    let effective_config = resolve_current_config()?;
    let json = matches!(show.format, scriberig_cli::RunFormat::Json);
    println!(
        "{}",
        scriberig_cli::config::render(&effective_config, json)?
    );
    Ok(())
}

/// Resolves configuration files and environment values without CLI overrides.
///
/// Effective settings with per-property provenance.
///
/// Returns a safe configuration-discovery or validation failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn resolve_current_config()
-> Result<scriberig_cli::config::ResolvedConfig, scriberig_cli::config::ConfigError> {
    let resolver = scriberig_cli::config::ConfigResolver::from_process().map_err(|error| {
        scriberig_cli::config::ConfigError {
            message: error.to_string(),
        }
    })?;
    resolver.resolve(&std::collections::BTreeMap::new())
}

/// Builds the upstream model and host credential adapter without reading secrets.
///
/// Upstream model, optional host credential adapter, and configured flag, or
/// no model when `base_url` is absent.
///
/// Returns safe configuration or credential-reference validation errors.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn resolve_provider_config()
-> Result<Option<ResolvedProviderSetup>, scriberig_cli::config::ConfigError> {
    let config = resolve_current_config()?;
    let Some(base_url) = config_optional_string(&config, "base_url") else {
        return Ok(None);
    };
    let reference = config_credential(&config)?;
    let credentials =
        authorized_credential_provider_for_check(reference.as_ref()).map_err(|error| {
            scriberig_cli::config::ConfigError {
                message: error.to_string(),
            }
        })?;
    let provider = config_string(&config, "provider");
    let model_id = config_string(&config, "model");
    let model = rpi_ai::Model::new(
        model_id.clone(),
        model_id,
        rpi_ai::Api::OpenaiCompletions,
        provider,
        base_url,
    );
    Ok(Some((model, credentials, reference.is_some())))
}

/// Resolves layered settings and applies them to one parsed run command.
///
/// - `args`: Parsed run or agent arguments, including explicit CLI options.
///
/// A copy of the arguments with winning configured values applied.
///
/// Returns an error when a discovered setting is malformed or invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn resolve_run_config(
    args: &scriberig_cli::RunArgs,
) -> Result<scriberig_cli::RunArgs, scriberig_cli::config::ConfigError> {
    let mut command_line = std::collections::BTreeMap::new();
    for name in [
        "db",
        "provider",
        "model",
        "base_url",
        "api_key_env",
        "credential_ref",
        "format",
        "no_stream",
    ] {
        if !args.explicit_cli.contains(name) {
            continue;
        }
        let value = match name {
            "credential_ref" if args.credential_ref.as_deref() == Some("none") => {
                serde_json::Value::Null
            }
            "credential_ref" => serde_json::to_value(
                scriberig_secrets::SecretRef::parse(
                    args.credential_ref.as_deref().unwrap_or_default(),
                )
                .map_err(|_| scriberig_cli::config::ConfigError {
                    message: "invalid credential reference".to_owned(),
                })?,
            )
            .map_err(|_| scriberig_cli::config::ConfigError {
                message: "invalid credential reference".to_owned(),
            })?,
            "db" => serde_json::Value::String(args.db.display().to_string()),
            "provider" => serde_json::Value::String(args.provider.clone()),
            "model" => serde_json::Value::String(args.model.clone()),
            "base_url" => args
                .base_url
                .clone()
                .map_or(serde_json::Value::Null, serde_json::Value::String),
            "api_key_env" => args
                .api_key_env
                .clone()
                .map_or(serde_json::Value::Null, serde_json::Value::String),
            "format" => serde_json::Value::String(
                match args.format {
                    scriberig_cli::RunFormat::Human => "human",
                    scriberig_cli::RunFormat::Json => "json",
                }
                .to_owned(),
            ),
            "no_stream" => serde_json::Value::Bool(args.no_stream),
            _ => continue,
        };
        command_line.insert(
            if name == "credential_ref" {
                "credential"
            } else {
                name
            }
            .to_owned(),
            value,
        );
    }

    let config_resolver =
        scriberig_cli::config::ConfigResolver::from_process().map_err(|error| {
            scriberig_cli::config::ConfigError {
                message: error.to_string(),
            }
        })?;
    let effective_config = config_resolver.resolve(&command_line)?;
    let mut effective = args.clone();
    effective.db = std::path::PathBuf::from(config_string(&effective_config, "db"));
    effective.provider = config_string(&effective_config, "provider");
    effective.model = config_string(&effective_config, "model");
    effective.base_url = config_optional_string(&effective_config, "base_url");
    effective.credential = config_credential(&effective_config)?;
    effective.api_key_env = match &effective.credential {
        Some(scriberig_secrets::SecretRef::Environment { name }) => Some(name.clone()),
        _ => None,
    };
    if effective.credential.is_some() && effective.base_url.is_none() {
        return Err(scriberig_cli::config::ConfigError {
            message: "a credential reference requires a provider base_url".to_owned(),
        });
    }
    effective.format = match config_string(&effective_config, "format").as_str() {
        "human" => scriberig_cli::RunFormat::Human,
        "json" => scriberig_cli::RunFormat::Json,
        value => {
            return Err(scriberig_cli::config::ConfigError {
                message: format!("invalid format `{value}`; expected human or json"),
            });
        }
    };
    effective.no_stream = effective_config.entries["no_stream"]
        .value
        .as_bool()
        .unwrap_or(false);
    Ok(effective)
}

/// Reads an atomic credential descriptor without resolving its secret value.
///
/// - `config`: Effective configuration with validated property types.
///
/// The selected reference, or no credential for an explicit null.
///
/// Returns a safe configuration error for an invalid reference.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn config_credential(
    config: &scriberig_cli::config::ResolvedConfig,
) -> Result<Option<scriberig_secrets::SecretRef>, scriberig_cli::config::ConfigError> {
    serde_json::from_value(config.entries["credential"].value.clone()).map_err(|_| {
        scriberig_cli::config::ConfigError {
            message: "invalid credential reference".to_owned(),
        }
    })
}

/// Composes a policy-checking credential provider without reading a secret.
///
/// - `args`: Effective provider settings and original explicit CLI markers.
///
/// A live authorization boundary or explicit unauthenticated execution.
///
/// Returns safe project, registry-location, or destination validation failures.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn authorized_credential_provider(
    args: &scriberig_cli::RunArgs,
) -> Result<Option<AuthorizedCredentialProvider>, scriberig_secrets::SecretError> {
    use scriberig_secrets::{Binding, ExecutionMode, SecretRef, SecretUseContext};
    let Some(reference) = &args.credential else {
        return Ok(None);
    };
    let base_url = args
        .base_url
        .as_deref()
        .ok_or(scriberig_secrets::SecretError::Invalid)?;
    let context = SecretUseContext {
        project_root: std::env::current_dir()
            .and_then(|root| root.canonicalize())
            .map_err(|_| scriberig_secrets::SecretError::Invalid)?,
        purpose: "model_provider".to_owned(),
        mode: ExecutionMode::Interactive,
    };
    let explicit_reference =
        args.explicit_cli.contains("api_key_env") || args.explicit_cli.contains("credential_ref");
    let invocation_binding = if explicit_reference
        && args.explicit_cli.contains("base_url")
        && matches!(reference, SecretRef::Environment { .. })
    {
        Some(Binding {
            credential: reference.clone(),
            base_url: scriberig_secrets::normalize_endpoint(base_url)?,
            project_root: context.project_root.clone(),
            purpose: context.purpose.clone(),
            mode: context.mode,
        })
    } else {
        None
    };
    build_authorized_credential_provider(reference, context, invocation_binding).map(Some)
}

/// Creates the ordinary live-policy credential source used by diagnostics.
///
/// - `reference`: Configured reference, or `None` for an unauthenticated API.
///
/// A credential source that checks user and system policy on each request.
///
/// Returns safe endpoint, path, or registry-location failures.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn authorized_credential_provider_for_check(
    reference: Option<&scriberig_secrets::SecretRef>,
) -> Result<Option<AuthorizedCredentialProvider>, scriberig_secrets::SecretError> {
    use scriberig_secrets::{ExecutionMode, SecretUseContext};
    let Some(reference) = reference else {
        return Ok(None);
    };
    let context = SecretUseContext {
        project_root: std::env::current_dir()
            .and_then(|root| root.canonicalize())
            .map_err(|_| scriberig_secrets::SecretError::Invalid)?,
        purpose: "model_provider".to_owned(),
        mode: ExecutionMode::Interactive,
    };
    build_authorized_credential_provider(reference, context, None).map(Some)
}

/// Wraps a reference with the selected identity and live policy locations.
///
/// - `reference`: Credential source to resolve only after authorization.
/// - `context`: Trusted project, purpose, and execution mode.
/// - `invocation_binding`: Optional grant created from explicit local CLI use.
///
/// An authorized runtime credential source.
///
/// Returns a safe registry-location or path failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn build_authorized_credential_provider(
    reference: &scriberig_secrets::SecretRef,
    context: scriberig_secrets::SecretUseContext,
    invocation_binding: Option<scriberig_secrets::Binding>,
) -> Result<AuthorizedCredentialProvider, scriberig_secrets::SecretError> {
    let provider = scriberig_secrets::registry::AuthorizedCredential {
        reference: reference.clone(),
        context,
        locations: scriberig_secrets::registry::RegistryLocations::from_process()?,
        invocation_binding,
        resolver: std::sync::Arc::new(scriberig_secrets::SourceResolver),
    };
    Ok(std::sync::Arc::new(provider))
}

/// Reads one required string setting from resolved configuration.
///
/// - `config`: Resolved property collection.
/// - `name`: Property name to read.
///
/// The property value as a string, or an empty string for null.
/// # Returns
/// The operation result.
fn config_string(config: &scriberig_cli::config::ResolvedConfig, name: &str) -> String {
    config.entries[name]
        .value
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

/// Reads one nullable string setting from resolved configuration.
///
/// - `config`: Resolved property collection.
/// - `name`: Property name to read.
///
/// The property string or `None` when it is null.
/// # Returns
/// The operation result.
fn config_optional_string(
    config: &scriberig_cli::config::ResolvedConfig,
    name: &str,
) -> Option<String> {
    config.entries[name].value.as_str().map(str::to_owned)
}
