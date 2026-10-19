//! Defines stable identifiers for long-running application operations.

use super::common::IdParseError;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Identifies one long-running operation across application events.
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
/// Stores a stable operation identity.
pub struct OperationId(String);

impl OperationId {
    /// Creates an operation identifier from a nonempty value.
    ///
    /// - `value`: Stable operation identity without the `op_` prefix.
    ///
    /// The operation identifier.
    ///
    /// Returns `EmptyName` when `value` is empty or whitespace-only.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(value: impl Into<String>) -> Result<Self, IdParseError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(IdParseError::EmptyName);
        }
        Ok(Self(value))
    }

    /// Borrows the unprefixed operation identity.
    ///
    /// - `self`: Operation identifier to inspect.
    ///
    /// The operation identity.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for OperationId {
    /// Formats the operation identifier with its stable prefix.
    ///
    /// - `self`: Operation identifier to format.
    /// - `formatter`: Destination formatter.
    ///
    /// A formatting result.
    ///
    /// Returns a formatting error when the destination rejects the write.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "op_{}", self.0)
    }
}

impl FromStr for OperationId {
    type Err = IdParseError;

    /// Parses a prefixed operation identifier.
    ///
    /// - `text`: Prefixed operation identity.
    ///
    /// The parsed operation identifier.
    ///
    /// Returns an error when the prefix is absent or the value is empty.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(
            text.strip_prefix("op_")
                .ok_or(IdParseError::InvalidFormat)?,
        )
    }
}
