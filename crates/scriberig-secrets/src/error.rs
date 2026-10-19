//! Defines secret-safe errors without backend diagnostic payloads.

use std::fmt;

/// Classifies failures without retaining secret or backend text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretError {
    /// The referenced secret does not exist.
    Missing,
    /// The operating-system store requires unlocking.
    Locked,
    /// The configured backend cannot be reached.
    Unavailable,
    /// Policy or filesystem permissions deny the operation.
    Denied,
    /// This platform does not support the requested operation.
    Unsupported,
    /// The reference, value, URL, or registry document is invalid.
    Invalid,
    /// An existing item cannot be overwritten without explicit replacement.
    Conflict,
    /// A lifecycle operation partially completed; authorization is disabled.
    Partial,
}

impl SecretError {
    /// Returns a stable process exit status for this failure.
    ///
    /// A distinct local secret-command exit status.
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Missing => 20,
            Self::Locked => 21,
            Self::Unavailable => 22,
            Self::Denied => 23,
            Self::Unsupported => 24,
            Self::Invalid => 25,
            Self::Conflict => 26,
            Self::Partial => 27,
        }
    }
}

impl fmt::Display for SecretError {
    /// Writes a fixed safe diagnostic.
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
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "secret is missing",
            Self::Locked => "secret store is locked",
            Self::Unavailable => "secret store is unavailable",
            Self::Denied => "secret access denied; register the exact destination and project binding",
            Self::Unsupported => "secret backend is unsupported on this platform",
            Self::Invalid => "invalid secret input, reference, destination, or policy",
            Self::Conflict => "secret already exists; use explicit replacement",
            Self::Partial => "secret operation partially completed; bindings disabled; inspect status before retrying",
        })
    }
}

impl std::error::Error for SecretError {}
