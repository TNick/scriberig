//! Defines fixed-width content digest identifiers.

// Import shared digest parsing and formatting helpers.
use super::common::{IdParseError, parse_hex, write_hex};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

// Generate separate wrapper types for digest identities.
macro_rules! digest_id {
    ($name:ident, $prefix:literal, $description:literal) => {
        #[doc = $description]
        ///
        /// # Public members
        /// - `new`: Constructs an identifier from a BLAKE3 digest.
        /// - `from_bytes`: Constructs an identifier from digest bytes.
        /// - `as_bytes`: Borrows the digest bytes.
        ///
        /// # Private members
        /// - `bytes`: Stores the fixed-width digest.
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name {
            bytes: [u8; 32],
        }

        // Provide digest construction and immutable byte access.
        impl $name {
            /// Constructs an identifier from its BLAKE3 digest bytes.
            ///
            /// # Parameters
            /// - `bytes`: Fixed-width BLAKE3 digest.
            ///
            /// # Returns
            /// The new identifier.
            #[must_use]
            pub const fn new(bytes: [u8; 32]) -> Self {
                Self { bytes }
            }

            /// Constructs an identifier from its fixed-width digest bytes.
            ///
            /// # Parameters
            /// - `bytes`: Fixed-width digest.
            ///
            /// # Returns
            /// The new identifier.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self::new(bytes)
            }

            /// Borrows the identifier's digest bytes.
            ///
            /// # Parameters
            /// - `self`: Identifier to inspect.
            ///
            /// # Returns
            /// The digest bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.bytes
            }
        }

        // Format a digest with its stable type-specific prefix.
        impl Display for $name {
            /// Formats the identifier as a type-prefixed lowercase hex digest.
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
            // Keep digest text lowercase and visibly type-specific.
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                write_hex($prefix, &self.bytes, formatter)
            }
        }

        // Accept only this digest type's stable prefixed representation.
        impl FromStr for $name {
            type Err = IdParseError;

            /// Parses a type-prefixed hexadecimal digest.
            ///
            /// # Parameters
            /// - `text`: Text to parse.
            ///
            /// # Returns
            /// The parsed identifier.
            ///
            /// # Errors
            /// Returns `InvalidFormat` for a wrong prefix, length, or hex digit.
            // Reject digest text whose prefix belongs to another identity type.
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Ok(Self::new(parse_hex($prefix, text)?))
            }
        }
    };
}

// Instantiate distinct immutable revision and snapshot digest IDs.
digest_id!(
    RevisionId,
    "rev_",
    "Identifies one exact immutable revision."
);
digest_id!(
    SnapshotId,
    "snp_",
    "Identifies one exact immutable snapshot."
);

// Verify digest round-trips, type separation, and byte identity.
#[cfg(test)]
mod tests {
    use super::{RevisionId, SnapshotId};
    use crate::ids::IdParseError;
    use std::str::FromStr;

    /// Checks display and parsing round-trips for both digest identifiers.
    // Check round-trip formatting for digest identifiers.
    #[test]

    fn digest_ids_round_trip() {
        let revision = RevisionId::new([0xabu8; 32]);
        let snapshot = SnapshotId::new([0x12u8; 32]);
        assert_eq!(RevisionId::from_str(&revision.to_string()), Ok(revision));
        assert_eq!(SnapshotId::from_str(&snapshot.to_string()), Ok(snapshot));
    }

    /// Rejects malformed text and a revision ID parsed as a snapshot ID.
    // Reject malformed and cross-type digest text.
    #[test]

    fn digest_parsing_rejects_invalid_text() {
        assert_eq!(
            RevisionId::from_str("rev_not-hex"),
            Err(IdParseError::InvalidFormat)
        );
        assert_eq!(
            SnapshotId::from_str(&RevisionId::new([0; 32]).to_string()),
            Err(IdParseError::InvalidFormat)
        );
    }

    /// Distinguishes digest identifiers with different underlying bytes.
    // Ensure distinct digest bytes produce distinct strongly typed IDs.
    #[test]

    fn different_digest_bytes_are_different_ids() {
        assert_ne!(RevisionId::new([0; 32]), RevisionId::new([1; 32]));
        assert_ne!(SnapshotId::new([0; 32]), SnapshotId::new([1; 32]));
    }
}
