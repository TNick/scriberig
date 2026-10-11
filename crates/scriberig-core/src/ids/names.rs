//! Defines validated names for roots and canonical object types.

// Import shared name validation errors.
use super::common::IdParseError;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

// Generate distinct validated UTF-8 name wrappers.
macro_rules! name_type {
    ($name:ident, $prefix:literal, $description:literal) => {
        #[doc = $description]
        ///
        /// # Public members
        /// - `new`: Validates and constructs a name.
        /// - `as_str`: Borrows the name text.
        ///
        /// # Private members
        /// - `value`: Stores the validated UTF-8 name.
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name {
            value: String,
        }

        // Validate names and expose read-only text access.
        impl $name {
            /// Validates and constructs a name.
            ///
            /// # Parameters
            /// - `value`: Name text.
            ///
            /// # Returns
            /// The validated name.
            ///
            /// # Errors
            /// Returns `EmptyName` when the text is empty or whitespace-only.
            // Reject names that cannot identify a meaningful root or type.
            pub fn new(value: impl Into<String>) -> Result<Self, IdParseError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(IdParseError::EmptyName);
                }
                Ok(Self { value })
            }

            /// Borrows the validated name text.
            ///
            /// # Parameters
            /// - `self`: Name to inspect.
            ///
            /// # Returns
            /// The stored name text.
            // Return the original validated text without rewriting it.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.value
            }
        }

        // Display the validated name with its type-specific prefix.
        impl Display for $name {
            /// Formats the name with its type-specific prefix.
            ///
            /// # Parameters
            /// - `self`: Name to format.
            /// - `formatter`: Destination formatter.
            ///
            /// # Returns
            /// A formatting result.
            ///
            /// # Errors
            /// Returns a formatting error if writing to the formatter fails.
            // Keep name output deterministic and distinguish wrapper types.
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                write!(formatter, concat!($prefix, "{}"), self.value)
            }
        }

        // Parse only correctly prefixed, nonempty names.
        impl FromStr for $name {
            type Err = IdParseError;

            /// Parses a prefixed name and validates its contents.
            ///
            /// # Parameters
            /// - `text`: Text to parse.
            ///
            /// # Returns
            /// The parsed name.
            ///
            /// # Errors
            /// Returns an error for a wrong prefix or empty name.
            // Validate the type prefix before constructing the wrapper.
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                let value = text
                    .strip_prefix($prefix)
                    .ok_or(IdParseError::InvalidFormat)?;
                Self::new(value)
            }
        }
    };
}

// Expose distinct validated root and object-type wrappers.
name_type!(
    RootName,
    "root_",
    "Names a mutable reference to a snapshot."
);
name_type!(ObjectType, "type_", "Names a canonical domain object type.");

// Verify name validation and distinct textual representations.
#[cfg(test)]
mod tests {
    use super::{ObjectType, RootName};
    use crate::ids::IdParseError;
    use std::str::FromStr;

    /// Checks name validation and prefixed parsing for both wrappers.
    // Reject empty names and check both wrapper round-trips.
    #[test]

    fn names_reject_empty_values_and_round_trip() {
        assert_eq!(RootName::new("  "), Err(IdParseError::EmptyName));
        let root = RootName::new("main").expect("valid root name");
        let object_type = ObjectType::new("rust.function").expect("valid type");
        assert_eq!(RootName::from_str(&root.to_string()), Ok(root));
        assert_eq!(
            ObjectType::from_str(&object_type.to_string()),
            Ok(object_type)
        );
    }
}
