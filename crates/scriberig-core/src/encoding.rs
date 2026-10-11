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
    pub fn write_tag(&mut self, tag: &[u8]) {
        self.bytes.extend_from_slice(tag);
    }

    /// Writes a 16-bit unsigned integer in big-endian order.
    ///
    /// # Parameters
    /// - `value`: Integer to encode.
    pub fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Writes an 8-bit unsigned integer.
    ///
    /// # Parameters
    /// - `value`: Integer to encode.
    pub fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    /// Writes a 32-bit unsigned integer in big-endian order.
    ///
    /// # Parameters
    /// - `value`: Integer to encode.
    pub fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Writes a 64-bit unsigned integer in big-endian order.
    ///
    /// # Parameters
    /// - `value`: Integer to encode.
    pub fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// Writes a byte string preceded by its 64-bit length.
    ///
    /// # Parameters
    /// - `value`: Bytes to encode.
    pub fn write_bytes(&mut self, value: &[u8]) {
        self.write_u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    /// Writes a UTF-8 string preceded by its byte length.
    ///
    /// # Parameters
    /// - `value`: String to encode.
    pub fn write_str(&mut self, value: &str) {
        self.write_bytes(value.as_bytes());
    }

    /// Writes a boolean using zero for false and one for true.
    ///
    /// # Parameters
    /// - `value`: Boolean to encode.
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
    pub fn write_option(&mut self, is_some: bool) {
        self.bytes.push(u8::from(is_some));
    }

    /// Writes an ordered sequence's element count.
    ///
    /// # Parameters
    /// - `length`: Number of sequence elements.
    pub fn write_sequence_len(&mut self, length: usize) {
        self.write_u64(length as u64);
    }

    /// Borrows the bytes written so far.
    ///
    /// # Returns
    /// The current canonical byte sequence.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the writer and returns the encoded bytes.
    ///
    /// # Returns
    /// The canonical byte sequence.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Encodes a value using explicit Scriberig canonical encoding rules.
pub trait CanonicalEncode {
    /// Appends this value's canonical representation to a writer.
    ///
    /// # Parameters
    /// - `self`: Value to encode.
    /// - `writer`: Output writer.
    fn encode_canonical(&self, writer: &mut CanonicalWriter);
}

/// Hashes a complete revision envelope into an immutable revision identity.
///
/// The domain tag, encoding version, object type, and payload all participate
/// in the digest so changes to any identity input produce a different value.
///
/// # Parameters
/// - `version`: Canonical encoding version.
/// - `object_type`: Semantic object type.
/// - `payload`: Already canonicalized payload bytes.
///
/// # Returns
/// The BLAKE3-derived revision identifier.
#[must_use]
pub fn revision_id(
    version: EncodingVersion,
    object_type: &crate::ObjectType,
    payload: &[u8],
) -> crate::RevisionId {
    let mut writer = CanonicalWriter::new();
    writer.write_tag(b"scriberig.revision.v1");
    writer.write_u16(version.value());
    writer.write_str(object_type.as_str());
    writer.write_bytes(payload);
    crate::RevisionId::from_bytes(*blake3::hash(writer.as_bytes()).as_bytes())
}

// Confirm primitive encodings and versions remain deterministic.
#[cfg(test)]
mod tests {
    use super::{CanonicalEncode, CanonicalWriter, EncodingVersion, revision_id};
    use crate::ObjectType;

    /// Checks canonical primitive output is repeatable.
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
        assert_eq!(
            first.as_bytes(),
            &[
                116, 101, 115, 116, 0, 1, 0, 0, 0, 0, 0, 0, 0, 3, 97, 98, 99, 1
            ]
        );
    }

    /// Verifies revision identity includes every envelope component.
    #[test]
    fn revision_hash_is_deterministic_and_domain_separated() {
        let object_type = ObjectType::new("markdown.paragraph").expect("valid type");
        let first = revision_id(EncodingVersion::V1, &object_type, b"payload");
        let second = revision_id(EncodingVersion::V1, &object_type, b"payload");
        assert_eq!(first, second);
        assert_ne!(
            first,
            revision_id(EncodingVersion::new(2), &object_type, b"payload")
        );
        let other_type = ObjectType::new("rust.function").expect("valid type");
        assert_ne!(
            first,
            revision_id(EncodingVersion::V1, &other_type, b"payload")
        );
        assert_ne!(
            first,
            revision_id(EncodingVersion::V1, &object_type, b"other")
        );
    }

    /// Checks independently constructed equivalent values encode identically.
    #[test]

    fn independent_values_encode_to_the_same_bytes() {
        /// Holds the scalar used by this canonical encoding test.
        #[derive(Clone, Copy)]
        struct Sample(u64);

        impl CanonicalEncode for Sample {
            /// Encodes the sample's scalar value.
            ///
            /// # Parameters
            /// - `self`: Sample to encode.
            /// - `writer`: Canonical byte output.
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
    #[test]
    fn encoding_version_one_has_stable_value() {
        assert_eq!(EncodingVersion::V1.value(), 1);
        assert_eq!(EncodingVersion::new(2).to_string(), "2");
    }
}
