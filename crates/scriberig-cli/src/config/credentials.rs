//! Normalizes compatibility aliases without merging secret-reference fields.

use super::ConfigError;
use scriberig_secrets::SecretRef;
use serde_json::Value;

/// Rejects multiple spellings of the credential property in one source layer.
///
/// - `names`: Property names supplied by one layer.
///
/// Unit when the layer has at most one credential spelling.
///
/// Returns a safe conflict diagnostic without property values.
pub(super) fn validate_aliases<'a>(
    names: impl IntoIterator<Item = &'a str>,
) -> Result<(), ConfigError> {
    let count = names
        .into_iter()
        .filter(|name| matches!(*name, "credential" | "api_key_env"))
        .count();
    if count > 1 {
        return Err(ConfigError {
            message: "credential and api_key_env cannot appear in the same configuration layer"
                .to_owned(),
        });
    }
    Ok(())
}

/// Normalizes a legacy environment property into one atomic reference.
///
/// - `name`: Original property name.
/// - `value`: Nonsecret reference value from that source.
///
/// The canonical property name and value.
///
/// Returns a safe validation failure without echoing raw input.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn normalize(name: String, value: Value) -> Result<(String, Value), ConfigError> {
    if name != "api_key_env" {
        return Ok((name, value));
    }
    let value = if value.is_null() {
        Value::Null
    } else {
        let name = value.as_str().ok_or_else(invalid)?;
        serde_json::json!({"source": "environment", "name": name})
    };
    validate(&value)?;
    Ok(("credential".to_owned(), value))
}

/// Validates a nullable credential reference without resolving its value.
///
/// - `value`: Atomic reference object or null.
///
/// Unit for supported references.
///
/// Returns a safe validation failure.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn validate(value: &Value) -> Result<(), ConfigError> {
    if value.is_null() {
        return Ok(());
    }
    let reference: SecretRef = serde_json::from_value(value.clone()).map_err(|_| invalid())?;
    reference.validate().map_err(|_| invalid())
}

/// Builds a diagnostic that cannot echo a mistaken inline secret.
///
/// A safe configuration error.
/// # Returns
/// The operation result.
fn invalid() -> ConfigError {
    ConfigError { message: "invalid credential reference; use keyring, environment, or an absolute protected file reference".to_owned() }
}
