//! Defines explicit deterministic encoding primitives for canonical values.

// Import the core identifier used to version canonical bytes.
use std::fmt::{self, Display, Formatter};

/// Selects the canonical encoding rules used for a value.
///
/// # Public members
/// - `new`: Constructs a version from its stable integer value.
/// - `value`: Returns the encoded version number.
///
/// # Private members
/// - `value`: Stores the version number.
// Represent the version of Scriberig's canonical byte format.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EncodingVersion {
    value: u16,
}

// Provide version construction and inspection.
impl EncodingVersion {
    /// The initial canonical encoding version.
    pub const V1: Self = Self { value: 1 };

    /// Constructs an encoding version from its stable integer value.
    ///
    /// # Parameters
    /// - `value`: Version number.
    ///
    /// # Returns
    /// The version wrapper.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self { value }
    }

    /// Returns the stable integer value of this version.
    ///
    /// # Parameters
    /// - `self`: Version to inspect.
    ///
    /// # Returns
    /// The version number.
    #[must_use]
    pub const fn value(self) -> u16 {
        self.value
    }
}

// Format encoding versions without locale dependence.
impl Display for EncodingVersion {
    /// Formats the version as its decimal number.
    ///
    /// # Parameters
    /// - `self`: Version to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// A formatting result.
    ///
    /// # Errors
    /// Returns a formatting error if writing to the formatter fails.
    // Render the version in its stable decimal form.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.value)
    }
}

/// Appends values to a deterministic, length-delimited byte sequence.
///
/// # Public members
/// - `new`: Creates an empty writer.
/// - `with_capacity`: Creates a writer with reserved space.
/// - `write_tag`: Writes a fixed domain tag.
/// - `write_u16`: Writes an unsigned integer in big-endian order.
/// - `write_u64`: Writes an unsigned integer in big-endian order.
/// - `write_bytes`: Writes a length-prefixed byte string.
/// - `write_str`: Writes a length-prefixed UTF-8 string.
/// - `write_bool`: Writes a single canonical boolean byte.
/// - `write_option`: Writes an option with explicit discriminant.
/// - `write_sequence_len`: Writes a sequence element count.
/// - `as_bytes`: Borrows the encoded bytes.
/// - `into_bytes`: Consumes the writer and returns encoded bytes.
///
/// # Private members
/// - `bytes`: Accumulates the canonical output.
// Provide an explicit writer for canonical primitives.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CanonicalWriter {
    bytes: Vec<u8>,
}

// Implement canonical primitive writing with explicit byte order.
impl CanonicalWriter {
    /// Creates an empty canonical writer.
    ///
    /// # Returns
    /// An empty writer.
    #[must_use]
    pub const fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Creates a writer with reserved output capacity.
    ///
    /// # Parameters
    /// - `capacity`: Initial byte capacity.
    ///
    /// # Returns
    /// An empty writer with reserved capacity.
    // Reserve output capacity before writing canonical data.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    /// Writes a caller-defined fixed domain tag.
    ///
    /// # Parameters
    /// - `tag`: Stable bytes identifying the value domain.
    // Append a fixed domain tag without an implicit platform format.
    pub fn write_tag(&mut self, tag: &[u8]) {
        self.bytes.extend_from_slice(tag);
    }

    /// Writes a 16-bit unsigned integer in big-endian order.
    ///
    /// # Parameters
    /// - `value`: Integer to encode.
    // Encode a small integer in network byte order.
    pub fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Writes a 64-bit unsigned integer in big-endian order.
    ///
    /// # Parameters
    /// - `value`: Integer to encode.
    // Encode a length or scalar in network byte order.
    pub fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Writes a byte string preceded by its 64-bit length.
    ///
    /// # Parameters
    /// - `value`: Bytes to encode.
    // Encode arbitrary bytes with an explicit length prefix.
    pub fn write_bytes(&mut self, value: &[u8]) {
        self.write_u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    /// Writes a UTF-8 string preceded by its byte length.
    ///
    /// # Parameters
    /// - `value`: String to encode.
    // Encode UTF-8 text with a byte-length prefix.
    pub fn write_str(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
    }

    /// Writes a boolean using zero for false and one for true.
    ///
    /// # Parameters
    /// - `value`: Boolean to encode.
    // Encode boolean values with explicit zero and one values.
    pub fn write_bool(&mut self, value: bool) {
        self.bytes.push(u8::from(value));
    }

    /// Writes an optional value's presence discriminant.
    ///
    /// Call `encode` only for `Some`; the caller encodes its contents with this
    /// writer after this method returns.
    ///
    /// # Parameters
    /// - `is_some`: Whether the optional value is present.
    // Encode option presence before the caller encodes its payload.
    pub fn write_option(&mut self, is_some: bool) {
        self.bytes.push(u8::from(is_some));
    }

    /// Writes an ordered sequence's element count.
    ///
    /// # Parameters
    /// - `length`: Number of sequence elements.
    // Encode a sequence length before its ordered elements.
    pub fn write_sequence_len(&mut self, length: usize) {
        self.write_u64(length as u64);
    }

    /// Borrows the bytes written so far.
    ///
    /// # Returns
    /// The current canonical byte sequence.
    // Expose encoded bytes without transferring ownership.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the writer and returns the encoded bytes.
    ///
    /// # Returns
    /// The canonical byte sequence.
    // Transfer the completed canonical byte buffer to the caller.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Encodes a value using explicit Scriberig canonical encoding rules.
// Define the interface for deterministic canonical value encoding.
pub trait CanonicalEncode {
    /// Appends this value's canonical representation to a writer.
    ///
    /// # Parameters
    /// - `self`: Value to encode.
    /// - `writer`: Output writer.
    // Append one value's canonical representation to a writer.
    fn encode_canonical(&self, writer: &mut CanonicalWriter);
}

// Confirm primitive encodings and versions remain deterministic.
#[cfg(test)]
mod tests {
    use super::{CanonicalEncode, CanonicalWriter, EncodingVersion};

    /// Checks canonical primitive output is repeatable.
    // Check stable length-delimited canonical primitive output.
    #[test]

    fn canonical_writer_encodes_primitives_deterministically() {
        let mut first = CanonicalWriter::new();
        first.write_tag(b"test");
        first.write_u16(1);
        first.write_str("abc");
        first.write_bool(true);

        let mut second = CanonicalWriter::new();
        second.write_tag(b"test");
        second.write_u16(1);
        second.write_str("abc");
        second.write_bool(true);

        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    /// Checks independently constructed equivalent values encode identically.
    // Check independently constructed values share canonical bytes.
    #[test]

    fn independent_values_encode_to_the_same_bytes() {
        /// Holds the scalar used by this canonical encoding test.
        // Define a test value with one canonical scalar field.
        #[derive(Clone, Copy)]
        struct Sample(u64);

        impl CanonicalEncode for Sample {
            /// Encodes the sample's scalar value.
            ///
            /// # Parameters
            /// - `self`: Sample to encode.
            /// - `writer`: Canonical byte output.
            // Encode the sample scalar as a canonical unsigned integer.
            fn encode_canonical(&self, writer: &mut CanonicalWriter) {
                writer.write_u64(self.0);
            }
        }

        let mut first = CanonicalWriter::new();
        Sample(42).encode_canonical(&mut first);
        let mut second = CanonicalWriter::new();
        Sample(42).encode_canonical(&mut second);
        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    /// Keeps the initial encoding version explicit and stable.
    // Ensure the initial encoding version is explicit and stable.
    #[test]
    fn encoding_version_one_has_stable_value() {
        assert_eq!(EncodingVersion::V1.value(), 1);
        assert_eq!(EncodingVersion::new(2).to_string(), "2");
    }
}
