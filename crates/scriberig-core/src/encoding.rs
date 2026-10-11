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

/// Reports invalid canonical encoding configuration.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EncodingError {
    /// The requested version is not implemented by this build.
    UnsupportedVersion {
        /// Version number that was requested.
        version: u16,
    },
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

    /// Validates and constructs a version supported by this implementation.
    ///
    /// # Parameters
    /// - `value`: Version number to validate.
    ///
    /// # Returns
    /// The supported encoding version.
    ///
    /// # Errors
    /// Returns `UnsupportedVersion` for versions not implemented here.
    pub const fn try_new(value: u16) -> Result<Self, EncodingError> {
        if value == Self::V1.value {
            Ok(Self::V1)
        } else {
            Err(EncodingError::UnsupportedVersion { version: value })
        }
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
/// - `write_unordered`: Writes items in canonical byte order.
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

    /// Writes an unordered collection after sorting each item's canonical bytes.
    ///
    /// # Parameters
    /// - `values`: Values whose semantic order is irrelevant.
    pub fn write_unordered<T: CanonicalEncode>(&mut self, values: impl IntoIterator<Item = T>) {
        let mut encoded = values
            .into_iter()
            .map(|value| {
                let mut writer = Self::new();
                value.encode_canonical(&mut writer);
                writer.into_bytes()
            })
            .collect::<Vec<_>>();
        encoded.sort();
        self.write_sequence_len(encoded.len());
        for value in encoded {
            self.write_bytes(&value);
        }
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

// Encode stable identifiers as fixed-width canonical byte values.
impl CanonicalEncode for crate::ids::EntityId {
    /// Encodes an entity identifier using its fixed-width bytes.
    ///
    /// # Parameters
    /// - `self`: Entity identifier to encode.
    /// - `writer`: Canonical byte output.
    fn encode_canonical(&self, writer: &mut CanonicalWriter) {
        writer.write_tag(b"scriberig.entity-id.v1");
        writer.write_bytes(self.as_bytes());
    }
}

// Encode immutable revision identifiers as fixed-width canonical byte values.
impl CanonicalEncode for crate::ids::RevisionId {
    /// Encodes a revision identifier using its fixed-width bytes.
    ///
    /// # Parameters
    /// - `self`: Revision identifier to encode.
    /// - `writer`: Canonical byte output.
    fn encode_canonical(&self, writer: &mut CanonicalWriter) {
        writer.write_tag(b"scriberig.revision-id.v1");
        writer.write_bytes(self.as_bytes());
    }
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
///
/// # Errors
/// Returns `UnsupportedVersion` when the encoding version is not implemented.
///
/// # Errors
/// Returns an error when the encoding version is unsupported.
pub fn revision_id(
    version: EncodingVersion,
    object_type: &crate::ObjectType,
    payload: &[u8],
) -> Result<crate::RevisionId, EncodingError> {
    Ok(crate::RevisionId::from_bytes(
        *blake3::hash(&revision_envelope(version, object_type, payload)?).as_bytes(),
    ))
}

/// Encodes the complete canonical revision envelope.
///
/// The bytes are a fixed domain tag, a big-endian version, a length-prefixed
/// object type, and a length-prefixed canonical payload. Callers that accept
/// externally supplied versions should validate them with `try_new` first.
///
/// # Parameters
/// - `version`: Canonical encoding version.
/// - `object_type`: Semantic object type.
/// - `payload`: Already canonicalized payload bytes.
///
/// # Returns
/// The deterministic canonical envelope bytes.
///
/// # Errors
/// Returns `UnsupportedVersion` when the encoding version is not implemented.
///
/// # Errors
/// Returns an error when the encoding version is unsupported.
pub fn revision_envelope(
    version: EncodingVersion,
    object_type: &crate::ObjectType,
    payload: &[u8],
) -> Result<Vec<u8>, EncodingError> {
    EncodingVersion::try_new(version.value())?;
    let mut writer = CanonicalWriter::new();
    writer.write_tag(b"scriberig.revision.v1");
    writer.write_u16(version.value());
    writer.write_str(object_type.as_str());
    writer.write_bytes(payload);
    Ok(writer.into_bytes())
}

#[cfg(test)]
#[path = "encoding_tests.rs"]
mod tests;
