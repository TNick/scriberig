//! Defines atomic secret references without embedded values.

use crate::SecretError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Identifies one credential source; variants never contain secret values.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum SecretRef {
    /// An application-namespaced operating-system credential.
    Keyring {
        /// Safe managed credential alias.
        name: String,
    },
    /// An explicitly authorized environment variable.
    Environment {
        /// Environment variable name, never its contents.
        name: String,
    },
    /// An explicitly authorized protected mounted file.
    File {
        /// Absolute path to a protected regular file.
        path: PathBuf,
    },
}

impl SecretRef {
    /// Parses an indirect command-line reference.
    ///
    /// - `text`: `keyring:NAME`, `environment:NAME`, or `file:ABSOLUTE_PATH`.
    ///
    /// A validated typed reference.
    ///
    /// Returns `Invalid` for unsupported schemes or invalid identifiers.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn parse(text: &str) -> Result<Self, SecretError> {
        let (kind, value) = text.split_once(':').ok_or(SecretError::Invalid)?;
        let reference = match kind {
            "keyring" => Self::Keyring {
                name: value.to_owned(),
            },
            "environment" => Self::Environment {
                name: value.to_owned(),
            },
            "file" => Self::File {
                path: PathBuf::from(value),
            },
            _ => return Err(SecretError::Invalid),
        };
        reference.validate()?;
        Ok(reference)
    }

    /// Validates a reference after configuration deserialization.
    ///
    /// Unit for a supported safe reference.
    ///
    /// Returns `Invalid` for unsafe identifiers or relative file paths.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn validate(&self) -> Result<(), SecretError> {
        let valid = match self {
            Self::Keyring { name } => {
                !name.is_empty()
                    && name.len() <= 128
                    && name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || b"/_-.".contains(&byte))
            }
            Self::Environment { name } => {
                !name.is_empty()
                    && name.len() <= 256
                    && name
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    && !name.as_bytes()[0].is_ascii_digit()
                    && !crate::process::is_toolchain_variable(name)
            }
            Self::File { path } => path.is_absolute(),
        };
        if valid {
            Ok(())
        } else {
            Err(SecretError::Invalid)
        }
    }
}
