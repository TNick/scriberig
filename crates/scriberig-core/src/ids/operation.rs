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
    /// # Parameters
    /// - `value`: Stable operation identity without the `op_` prefix.
    ///
    /// # Returns
    /// The operation identifier.
    ///
    /// # Errors
    /// Returns `EmptyName` when `value` is empty or whitespace-only.
    pub fn new(value: impl Into<String>) -> Result<Self, IdParseError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(IdParseError::EmptyName);
        }
        Ok(Self(value))
    }

    /// Borrows the unprefixed operation identity.
    ///
    /// # Parameters
    /// - `self`: Operation identifier to inspect.
    ///
    /// # Returns
    /// The operation identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for OperationId {
    /// Formats the operation identifier with its stable prefix.
    ///
    /// # Parameters
    /// - `self`: Operation identifier to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects the write.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "op_{}", self.0)
    }
}

impl FromStr for OperationId {
    type Err = IdParseError;

    /// Parses a prefixed operation identifier.
    ///
    /// # Parameters
    /// - `text`: Prefixed operation identity.
    ///
    /// # Returns
    /// The parsed operation identifier.
    ///
    /// # Errors
    /// Returns an error when the prefix is absent or the value is empty.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(
            text.strip_prefix("op_")
                .ok_or(IdParseError::InvalidFormat)?,
        )
    }
}
