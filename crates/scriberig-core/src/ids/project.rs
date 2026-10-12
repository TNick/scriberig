//! Defines stable identity for an onboarded project.

use super::common::{IdParseError, parse_hex, write_hex};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

#[derive(
    Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
/// Identifies an onboarded project independently of its current filesystem path.
///
/// # Public members
/// - `fresh`: Generates a new project identity.
/// - `from_bytes`: Constructs an identity from fixed-width bytes.
/// - `as_bytes`: Borrows the identity bytes.
///
/// # Private members
/// - `bytes`: Stores the fixed-width identity.
pub struct ProjectId {
    /// Fixed-width bytes containing the project identity.
    bytes: [u8; 16],
}

impl ProjectId {
    /// Generates a fresh project identity.
    ///
    /// # Returns
    /// A new project identity.
    #[must_use]
    pub fn fresh() -> Self {
        Self::from_bytes(uuid::Uuid::new_v4().into_bytes())
    }

    /// Constructs a project identity from fixed-width bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self { bytes }
    }

    /// Borrows the project identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }
}

impl Display for ProjectId {
    /// Formats the project identity with its stable prefix.
    ///
    /// # Parameters
    /// - `self`: Project identity to format.
    /// - `formatter`: Formatting destination.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error when the destination rejects output.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write_hex("prj_", &self.bytes, formatter)
    }
}

impl FromStr for ProjectId {
    type Err = IdParseError;

    /// Parses a prefixed hexadecimal project identity.
    ///
    /// # Parameters
    /// - `text`: Text to parse.
    ///
    /// # Returns
    /// The parsed project identity.
    ///
    /// # Errors
    /// Returns an identifier format error for malformed text.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_bytes(parse_hex("prj_", text)?))
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectId;
    use std::str::FromStr;

    /// Verifies project identities have deterministic text round-trips.
    #[test]
    fn project_id_round_trip() {
        let project = ProjectId::from_bytes([0x5a; 16]);
        assert_eq!(ProjectId::from_str(&project.to_string()), Ok(project));
    }
}
