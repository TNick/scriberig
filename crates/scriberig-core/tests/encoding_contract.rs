//! Verifies canonical revision envelope fixtures and version validation.

use std::collections::BTreeMap;

use scriberig_core::{
    CanonicalEncode, CanonicalWriter, EncodingError, EncodingVersion, KnowledgeValue, ObjectType,
    revision_envelope, revision_id,
};

/// Locks the initial revision envelope bytes as a golden fixture.
#[test]
fn revision_envelope_has_stable_bytes() {
    let object_type = ObjectType::new("markdown.paragraph").expect("valid type");
    assert_eq!(
        revision_envelope(EncodingVersion::V1, &object_type, b"payload"),
        Ok(vec![
            115, 99, 114, 105, 98, 101, 114, 105, 103, 46, 114, 101, 118, 105, 115, 105, 111, 110,
            46, 118, 49, 0, 1, 0, 0, 0, 0, 0, 0, 0, 18, 109, 97, 114, 107, 100, 111, 119, 110, 46,
            112, 97, 114, 97, 103, 114, 97, 112, 104, 0, 0, 0, 0, 0, 0, 0, 7, 112, 97, 121, 108,
            111, 97, 100,
        ])
    );
}

/// Rejects versions that have no implemented decoding rules.
#[test]
fn unsupported_encoding_versions_fail_explicitly() {
    assert_eq!(
        EncodingVersion::try_new(2),
        Err(EncodingError::UnsupportedVersion { version: 2 })
    );
    let object_type = ObjectType::new("test.item").expect("valid type");
    assert_eq!(
        revision_envelope(EncodingVersion::new(2), &object_type, b"payload"),
        Err(EncodingError::UnsupportedVersion { version: 2 })
    );
}

/// Proves independently constructed canonical values receive the same ID.
#[test]
fn independent_values_produce_the_same_revision_id() {
    let object_type = ObjectType::new("test.value").expect("valid type");
    let first = KnowledgeValue::Record(BTreeMap::from([(
        "answer".to_owned(),
        KnowledgeValue::UnsignedInteger(42),
    )]));
    let second = KnowledgeValue::Record(BTreeMap::from([(
        "answer".to_owned(),
        KnowledgeValue::UnsignedInteger(42),
    )]));

    let first_id =
        revision_id(EncodingVersion::V1, &object_type, &encode(&first)).expect("supported version");
    let second_id = revision_id(EncodingVersion::V1, &object_type, &encode(&second))
        .expect("supported version");
    assert_eq!(first_id, second_id);
}

/// Proves ordered values preserve order in canonical identity.
#[test]
fn ordered_collection_order_changes_revision_id() {
    let object_type = ObjectType::new("test.list").expect("valid type");
    let first = KnowledgeValue::List(vec![
        KnowledgeValue::UnsignedInteger(1),
        KnowledgeValue::UnsignedInteger(2),
    ]);
    let second = KnowledgeValue::List(vec![
        KnowledgeValue::UnsignedInteger(2),
        KnowledgeValue::UnsignedInteger(1),
    ]);

    assert_ne!(
        revision_id(EncodingVersion::V1, &object_type, &encode(&first)),
        revision_id(EncodingVersion::V1, &object_type, &encode(&second))
    );
}

/// Proves unordered values are canonicalized before encoding.
#[test]
fn unordered_collection_order_does_not_change_bytes() {
    let mut first = CanonicalWriter::new();
    first.write_unordered([
        KnowledgeValue::String("b".to_owned()),
        KnowledgeValue::String("a".to_owned()),
    ]);
    let mut second = CanonicalWriter::new();
    second.write_unordered([
        KnowledgeValue::String("a".to_owned()),
        KnowledgeValue::String("b".to_owned()),
    ]);

    assert_eq!(first.into_bytes(), second.into_bytes());
}

/// Encodes a value through the production canonical value implementation.
///
/// - `value`: Value to encode.
///
/// The canonical bytes for `value`.
/// # Returns
/// The operation result.
fn encode(value: &KnowledgeValue) -> Vec<u8> {
    let mut writer = CanonicalWriter::new();
    value.encode_canonical(&mut writer);
    writer.into_bytes()
}
