//! Defines unprefixed, validated semantic type identifiers.

use super::common::IdParseError;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
/// Names a semantic object type without coupling it to a registry.
///
/// # Public members
/// - `new`: Validates and constructs a type identifier.
/// - `as_str`: Borrows the identifier text.
///
/// # Private members
/// - `value`: Stores the validated UTF-8 identifier.
pub struct TypeId {
    /// Validated semantic type name.
    value: String,
}

impl TypeId {
    /// Validates and constructs a semantic type identifier.
    ///
    /// - `value`: Nonempty UTF-8 type name.
    ///
    /// The validated type identifier.
    ///
    /// Returns `EmptyName` when the identifier is empty or whitespace-only.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(value: impl Into<String>) -> Result<Self, IdParseError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(IdParseError::EmptyName);
        }
        Ok(Self { value })
    }

    /// Borrows the validated identifier text.
    ///
    /// The exact UTF-8 text supplied at construction.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Display for TypeId {
    /// Formats the identifier without adding a registry-specific prefix.
    ///
    /// - `self`: Identifier to format.
    /// - `formatter`: Destination formatter.
    ///
    /// A formatting result.
    ///
    /// Returns a formatting error if writing to the formatter fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value)
    }
}

impl FromStr for TypeId {
    type Err = IdParseError;

    /// Parses a nonempty semantic type identifier.
    ///
    /// - `text`: Identifier text.
    ///
    /// The validated type identifier.
    ///
    /// Returns `EmptyName` when the text is empty or whitespace-only.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::new(text)
    }
}

#[cfg(test)]
mod tests {
    use super::TypeId;
    use crate::IdParseError;
    use std::str::FromStr;

    /// Checks exact display and parsing for a valid semantic type.
    #[test]
    fn type_id_round_trip() {
        let type_id = TypeId::new("markdown.paragraph").expect("valid type");
        assert_eq!(type_id.to_string(), "markdown.paragraph");
        assert_eq!(TypeId::from_str(&type_id.to_string()), Ok(type_id));
    }

    /// Rejects empty semantic type identifiers.
    #[test]
    fn type_id_rejects_empty_text() {
        assert_eq!(TypeId::new("  "), Err(IdParseError::EmptyName));
    }
}
