//! Defines unprefixed, validated semantic type identifiers.

use super::common::IdParseError;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Names a semantic object type without coupling it to a registry.
///
/// # Public members
/// - `new`: Validates and constructs a type identifier.
/// - `as_str`: Borrows the identifier text.
///
/// # Private members
/// - `value`: Stores the validated UTF-8 identifier.
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct TypeId {
    value: String,
}

impl TypeId {
    /// Validates and constructs a semantic type identifier.
    ///
    /// # Parameters
    /// - `value`: Nonempty UTF-8 type name.
    ///
    /// # Returns
    /// The validated type identifier.
    ///
    /// # Errors
    /// Returns `EmptyName` when the identifier is empty or whitespace-only.
    pub fn new(value: impl Into<String>) -> Result<Self, IdParseError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(IdParseError::EmptyName);
        }
        Ok(Self { value })
    }

    /// Borrows the validated identifier text.
    ///
    /// # Returns
    /// The exact UTF-8 text supplied at construction.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Display for TypeId {
    /// Formats the identifier without adding a registry-specific prefix.
    ///
    /// # Parameters
    /// - `self`: Identifier to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error if writing to the formatter fails.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value)
    }
}

impl FromStr for TypeId {
    type Err = IdParseError;

    /// Parses a nonempty semantic type identifier.
    ///
    /// # Parameters
    /// - `text`: Identifier text.
    ///
    /// # Returns
    /// The validated type identifier.
    ///
    /// # Errors
    /// Returns `EmptyName` when the text is empty or whitespace-only.
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
