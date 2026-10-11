//! Provides shared identifier formatting, parsing, and errors.

// Import formatter primitives used by identifier helpers.
use std::fmt::{self, Formatter};

/// Reports malformed or invalid textual identifier values.
///
/// # Public members
/// - `InvalidFormat`: Input text has the wrong prefix, length, or alphabet.
/// - `EmptyName`: A root or object type name is empty or whitespace-only.
///
/// # Private members
/// None.
// Describe failures while parsing stable public identifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdParseError {
    /// The text does not match the identifier's required representation.
    InvalidFormat,
    /// The name must contain at least one non-whitespace character.
    EmptyName,
}

// Format parse errors without depending on a backend or locale.
impl fmt::Display for IdParseError {
    /// Formats the parse error for diagnostics.
    ///
    /// # Parameters
    /// - `self`: Error to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error if writing to the formatter fails.
    // Keep diagnostics stable and independent of storage backends.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => formatter.write_str("invalid identifier format"),
            Self::EmptyName => formatter.write_str("name must not be empty"),
        }
    }
}

// Make parse failures usable with standard error handling.
impl std::error::Error for IdParseError {}

/// Writes a type prefix and fixed-width lowercase hexadecimal bytes.
///
/// # Parameters
/// - `prefix`: Type-specific textual prefix.
/// - `bytes`: Bytes to format.
/// - `formatter`: Destination formatter.
///
/// # Returns
/// A formatting result.
///
/// # Errors
/// Returns a formatting error if writing to the formatter fails.
// Format fixed-width byte arrays as lowercase hexadecimal.
pub fn write_hex(prefix: &str, bytes: &[u8], formatter: &mut Formatter<'_>) -> fmt::Result {
    formatter.write_str(prefix)?;
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

/// Parses exactly the expected number of prefixed hexadecimal bytes.
///
/// # Parameters
/// - `prefix`: Required type-specific prefix.
/// - `text`: Text to parse.
///
/// # Returns
/// A fixed-width byte array.
///
/// # Errors
/// Returns `InvalidFormat` when the prefix, digit count, or alphabet is invalid.
// Parse prefixed hexadecimal bytes without panicking.
pub(super) fn parse_hex<const N: usize>(prefix: &str, text: &str) -> Result<[u8; N], IdParseError> {
    let digits = text
        .strip_prefix(prefix)
        .ok_or(IdParseError::InvalidFormat)?;
    if digits.len() != N * 2 || !digits.is_ascii() {
        return Err(IdParseError::InvalidFormat);
    }
    let mut bytes = [0; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&digits[start..start + 2], 16)
            .map_err(|_| IdParseError::InvalidFormat)?;
    }
    Ok(bytes)
}
