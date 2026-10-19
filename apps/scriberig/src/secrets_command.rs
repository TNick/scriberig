//! Implements local secret lifecycle operations without exposing stored values.

use scriberig_cli::secrets::{BindingArgs, SecretsArgs, SecretsCommand};
use scriberig_secrets::registry::RegistryLocations;
use scriberig_secrets::{
    Binding, ExecutionMode, OsSecretStore, SecretError, SecretRef, SecretStore, SecretUseContext,
    SecretValue,
};
use std::io::IsTerminal;

/// Executes one local administrative command using the native credential store.
///
/// - `args`: Parsed command with references and metadata only.
///
/// Unit after the safe command result is printed.
///
/// Returns a safe structured lifecycle or backend failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn run(args: &SecretsArgs) -> Result<(), SecretError> {
    let locations = RegistryLocations::from_process()?;
    let store = OsSecretStore;
    match &args.command {
        SecretsCommand::Set {
            name,
            stdin,
            replace,
        } => {
            SecretRef::Keyring { name: name.clone() }.validate()?;
            let value = if *stdin {
                scriberig_secrets::files::read_secret(std::io::stdin().lock())?
            } else {
                if !std::io::stdin().is_terminal() {
                    return Err(SecretError::Invalid);
                }
                let text =
                    rpassword::prompt_password("Secret: ").map_err(|_| SecretError::Unavailable)?;
                SecretValue::new(text)?
            };
            locations.lock()?.set(&store, name, &value, *replace)?;
            println!("secret stored");
        }
        SecretsCommand::List { format } => {
            let registry = locations.load()?;
            let value =
                serde_json::json!({"names": registry.names, "bindings": registry.policy.bindings});
            print_metadata(&value, *format)?;
        }
        SecretsCommand::Status { name, format } => {
            let available = store.exists(name)?;
            print_metadata(
                &serde_json::json!({"name": name, "backend": "keyring", "status": if available { "available" } else { "missing" }}),
                *format,
            )?;
            if !available {
                return Err(SecretError::Missing);
            }
        }
        SecretsCommand::Delete { name } => {
            locations.lock()?.delete(&store, name)?;
            println!("secret and bindings removed");
        }
        SecretsCommand::Bind(args) => {
            let binding = binding(args)?;
            if let Some(policy) = locations.system()? {
                let context = SecretUseContext {
                    project_root: binding.project_root.clone(),
                    purpose: binding.purpose.clone(),
                    mode: binding.mode,
                };
                policy.authorize(&binding.credential, &binding.base_url, &context)?;
            }
            locations.lock()?.bind(binding)?;
            println!("credential binding saved");
        }
        SecretsCommand::Unbind(args) => {
            locations.lock()?.unbind(binding(args)?)?;
            println!("credential binding removed");
        }
    }
    Ok(())
}

/// Constructs an exact destination/project binding from explicit local intent.
///
/// - `args`: Explicit administrative reference and destination arguments.
///
/// A normalized binding with canonical project scope.
///
/// Returns a safe invalid-reference or project-path failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn binding(args: &BindingArgs) -> Result<Binding, SecretError> {
    let reference = if args.name.contains(':') {
        SecretRef::parse(&args.name)?
    } else {
        SecretRef::Keyring {
            name: args.name.clone(),
        }
    };
    reference.validate()?;
    Ok(Binding {
        credential: reference,
        base_url: scriberig_secrets::normalize_endpoint(&args.base_url)?,
        project_root: args
            .project_root
            .canonicalize()
            .map_err(|_| SecretError::Invalid)?,
        purpose: "model_provider".to_owned(),
        mode: if args.service {
            ExecutionMode::Service
        } else {
            ExecutionMode::Interactive
        },
    })
}

/// Prints metadata without ever receiving secret material.
///
/// - `value`: Secret-free metadata object.
/// - `format`: Human-readable or JSON representation.
///
/// Unit after writing the result.
///
/// Returns a safe serialization error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn print_metadata(
    value: &serde_json::Value,
    format: scriberig_cli::RunFormat,
) -> Result<(), SecretError> {
    let text = if matches!(format, scriberig_cli::RunFormat::Json) {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
    .map_err(|_| SecretError::Invalid)?;
    println!("{text}");
    Ok(())
}
