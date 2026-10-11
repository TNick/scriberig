//! Defines stable entity identity as a fixed-width value.

// Import shared parsing and formatting helpers.
use super::common::{IdParseError, parse_hex, write_hex};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Identifies a stable conceptual entity independently of its revisions.
///
/// # Public members
/// - `new`: Constructs an entity identifier from 16 bytes.
/// - `from_bytes`: Constructs an entity identifier from fixed-width bytes.
/// - `as_bytes`: Borrows the identifier bytes.
///
/// # Private members
/// - `bytes`: Stores the identifier's 128-bit value.
#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub struct EntityId {
    bytes: [u8; 16],
}

// Implement construction and byte access for entity identifiers.
impl EntityId {
    /// Generates a fresh non-content-derived entity identifier.
    ///
    /// # Returns
    /// A new randomly generated entity identifier.
    #[must_use]
    pub fn fresh() -> Self {
        Self::new(uuid::Uuid::new_v4().into_bytes())
    }

    /// Constructs an entity identifier from its 128-bit value.
    ///
    /// # Parameters
    /// - `bytes`: Fixed-width identifier bytes.
    ///
    /// # Returns
    /// The new entity identifier.
    #[must_use]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self { bytes }
    }

    /// Constructs an entity identifier from its fixed-width bytes.
    ///
    /// # Parameters
    /// - `bytes`: Fixed-width identifier bytes.
    ///
    /// # Returns
    /// The new entity identifier.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self::new(bytes)
    }

    /// Borrows the identifier's fixed-width bytes.
    ///
    /// # Parameters
    /// - `self`: Identifier to inspect.
    ///
    /// # Returns
    /// The identifier bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }
}

// Provide deterministic hexadecimal display for entity identifiers.
impl Display for EntityId {
    /// Formats the entity identifier with its type-specific prefix.
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
        write_hex("ent_", &self.bytes, formatter)
    }
}

// Parse entity identifiers only from their own prefixed representation.
impl FromStr for EntityId {
    type Err = IdParseError;

    /// Parses a prefixed hexadecimal entity identifier.
    ///
    /// # Parameters
    /// - `text`: Text to parse.
    ///
    /// # Returns
    /// The parsed identifier.
    ///
    /// # Errors
    /// Returns `InvalidFormat` for a wrong prefix, length, or hexadecimal digit.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(parse_hex("ent_", text)?))
    }
}

// Verify entity formatting, parsing, and fixed-width storage.
#[cfg(test)]
mod tests {
    use super::EntityId;
    use std::str::FromStr;

    /// Checks round-trip display and parsing for entity identifiers.
    #[test]

    fn entity_id_round_trip() {
        let entity = EntityId::new([0x5a; 16]);
        assert_eq!(EntityId::from_str(&entity.to_string()), Ok(entity));
    }

    /// Checks fresh entity identifiers are independently generated.
    #[test]
    fn fresh_entity_ids_are_distinct() {
        assert_ne!(EntityId::fresh(), EntityId::fresh());
    }
}
