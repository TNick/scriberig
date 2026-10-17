//! Resolves explicit environment/file sources and native credential stores.

use crate::{SecretError, SecretRef, SecretValue};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use keyring as native;
#[cfg(windows)]
use keyring_core as native;

/// Reads a referenced secret without implicitly selecting a fallback source.
pub trait SecretResolver: Send + Sync {
    /// Reads one explicitly authorized reference.
    ///
    /// # Parameters
    /// - `reference`: Source already authorized by the caller.
    ///
    /// # Returns
    /// A protected credential buffer.
    ///
    /// # Errors
    /// Returns a classified safe backend failure.
    fn resolve(&self, reference: &SecretRef) -> Result<SecretValue, SecretError>;
}

/// Manages only Scriberig-namespaced operating-system credentials.
pub trait SecretStore: Send + Sync {
    /// Reads one managed item.
    ///
    /// # Parameters
    /// - `name`: Validated credential alias.
    ///
    /// # Returns
    /// The protected credential.
    ///
    /// # Errors
    /// Returns safe missing, locked, or unavailable errors.
    fn get(&self, name: &str) -> Result<SecretValue, SecretError>;

    /// Writes or replaces one managed item.
    ///
    /// # Parameters
    /// - `name`: Validated credential alias.
    /// - `value`: Protected credential to persist.
    ///
    /// # Returns
    /// Unit once the backend write completes.
    ///
    /// # Errors
    /// Returns a safe backend failure.
    fn set(&self, name: &str, value: &SecretValue) -> Result<(), SecretError>;

    /// Deletes one managed item.
    ///
    /// # Parameters
    /// - `name`: Validated credential alias.
    ///
    /// # Returns
    /// Unit after deletion.
    ///
    /// # Errors
    /// Returns a safe backend failure.
    fn delete(&self, name: &str) -> Result<(), SecretError>;

    /// Checks metadata availability without returning secret contents.
    ///
    /// # Parameters
    /// - `name`: Validated credential alias.
    ///
    /// # Returns
    /// Whether the item exists.
    ///
    /// # Errors
    /// Returns a safe locked, denied, or unavailable failure.
    fn exists(&self, name: &str) -> Result<bool, SecretError>;
}

/// Uses the native OS store with no plaintext fallback.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsSecretStore;

/// Builds an application-namespaced native credential handle.
///
/// # Parameters
/// - `name`: Managed credential alias.
///
/// # Returns
/// A handle that has not retrieved any secret.
///
/// # Errors
/// Returns a safe reference or backend failure.
fn entry(name: &str) -> Result<native::Entry, SecretError> {
    SecretRef::Keyring {
        name: name.to_owned(),
    }
    .validate()?;
    #[cfg(windows)]
    {
        use keyring_core::api::CredentialStoreApi;
        let store = windows_native_keyring_store::Store::new().map_err(classify)?;
        let modifiers = std::collections::HashMap::from([("persistence", "Local")]);
        store
            .build("scriberig.secrets.v1", name, Some(&modifiers))
            .map_err(classify)
    }
    #[cfg(not(windows))]
    {
        native::Entry::new("scriberig.secrets.v1", name).map_err(classify)
    }
}

/// Removes backend payloads from public error diagnostics.
///
/// # Parameters
/// - `error`: Native store error; its diagnostic text is never retained.
///
/// # Returns
/// A safe Scriberig error category.
fn classify(error: native::Error) -> SecretError {
    let category = match &error {
        native::Error::NoEntry => SecretError::Missing,
        native::Error::NoStorageAccess(_) => SecretError::Locked,
        native::Error::Invalid(_, _) | native::Error::TooLong(_, _) => SecretError::Invalid,
        _ => SecretError::Unavailable,
    };
    drop(error);
    category
}

impl SecretStore for OsSecretStore {
    /// Reads a protected value from the native store.
    ///
    /// # Parameters
    /// - `name`: Managed alias.
    ///
    /// # Returns
    /// A protected value.
    ///
    /// # Errors
    /// Returns safe backend errors.
    fn get(&self, name: &str) -> Result<SecretValue, SecretError> {
        SecretValue::new(entry(name)?.get_password().map_err(classify)?)
    }

    /// Writes a protected value to the native store.
    ///
    /// # Parameters
    /// - `name`: Managed alias.
    /// - `value`: Protected value to persist.
    ///
    /// # Returns
    /// Unit after writing.
    ///
    /// # Errors
    /// Returns safe backend errors.
    fn set(&self, name: &str, value: &SecretValue) -> Result<(), SecretError> {
        entry(name)?.set_password(value.expose()).map_err(classify)
    }

    /// Removes a managed native item.
    ///
    /// # Parameters
    /// - `name`: Managed alias.
    ///
    /// # Returns
    /// Unit after deleting.
    ///
    /// # Errors
    /// Returns safe backend errors.
    fn delete(&self, name: &str) -> Result<(), SecretError> {
        entry(name)?.delete_credential().map_err(classify)
    }

    /// Queries metadata without retrieving the secret.
    ///
    /// # Parameters
    /// - `name`: Managed alias.
    ///
    /// # Returns
    /// Whether the item exists.
    ///
    /// # Errors
    /// Returns safe backend errors.
    fn exists(&self, name: &str) -> Result<bool, SecretError> {
        match entry(name)?.get_attributes() {
            Ok(_) => Ok(true),
            Err(native::Error::NoEntry) => Ok(false),
            Err(error) => Err(classify(error)),
        }
    }
}

/// Resolves precisely the source variant selected by configuration.
#[derive(Clone, Copy, Debug, Default)]
pub struct SourceResolver;

impl SecretResolver for SourceResolver {
    /// Reads one explicit source without fallback.
    ///
    /// # Parameters
    /// - `reference`: Previously authorized credential reference.
    ///
    /// # Returns
    /// A protected secret value.
    ///
    /// # Errors
    /// Returns safe source validation or availability errors.
    fn resolve(&self, reference: &SecretRef) -> Result<SecretValue, SecretError> {
        reference.validate()?;
        match reference {
            SecretRef::Keyring { name } => OsSecretStore.get(name),
            SecretRef::Environment { name } => {
                SecretValue::new(std::env::var(name).map_err(|_| SecretError::Missing)?)
            }
            SecretRef::File { path } => crate::files::read_secret_file(path),
        }
    }
}
