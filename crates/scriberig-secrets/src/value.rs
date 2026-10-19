//! Holds nonserializable zeroizing credential buffers.

use crate::SecretError;
use secrecy::{ExposeSecret, SecretString};

/// Owns one protected credential without serialization or implicit cloning.
///
/// # Public members
/// - `new`: Validates an owned credential.
/// - `expose`: Borrows the credential at its use boundary.
///
/// # Private members
/// - `0`: Zeroizing secret string.
pub struct SecretValue(SecretString);

impl SecretValue {
    /// Takes ownership of a secret and validates the bearer-token input.
    ///
    /// - `value`: Secret string whose allocation will be zeroized on drop.
    ///
    /// A validated protected buffer.
    ///
    /// Returns `Invalid` for empty, oversized, or multiline input.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(value: String) -> Result<Self, SecretError> {
        let protected = SecretString::from(value);
        let text = protected.expose_secret();
        if text.is_empty() || text.len() > 65_536 || text.contains(['\r', '\n', '\0']) {
            return Err(SecretError::Invalid);
        }
        Ok(Self(protected))
    }

    /// Borrows the value explicitly at a storage or transport boundary.
    ///
    /// A borrowed secret that must never enter ordinary diagnostic output.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl std::fmt::Debug for SecretValue {
    /// Formats a fixed redaction marker.
    ///
    /// - `formatter`: Destination formatter.
    ///
    /// Formatting success.
    ///
    /// Returns formatter failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SecretValue([REDACTED])")
    }
}
