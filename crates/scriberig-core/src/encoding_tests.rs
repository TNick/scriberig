//! Tests canonical encoding and revision identity invariants.

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
    let first =
        revision_id(EncodingVersion::V1, &object_type, b"payload").expect("supported version");
    let second =
        revision_id(EncodingVersion::V1, &object_type, b"payload").expect("supported version");
    assert_eq!(first, second);
    assert_ne!(
        first,
        revision_id(EncodingVersion::V1, &object_type, b"other").expect("supported version")
    );
    let other_type = ObjectType::new("rust.function").expect("valid type");
    assert_ne!(
        first,
        revision_id(EncodingVersion::V1, &other_type, b"payload").expect("supported version")
    );
    assert_eq!(
        revision_id(EncodingVersion::new(2), &object_type, b"payload"),
        Err(super::EncodingError::UnsupportedVersion { version: 2 })
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
