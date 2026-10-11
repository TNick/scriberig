//! Verifies immutable revision storage behavior.

use scriberig_core::{EncodingVersion, ObjectType, revision_id};
use scriberig_store::{Revision, RevisionStore};
use scriberig_store_memory::MemoryRevisionStore;

/// Creates a deterministic revision fixture.
///
/// # Parameters
/// - `payload`: Canonical payload bytes.
/// - `object`: Object type identifier.
///
/// # Returns
/// An immutable revision fixture.
fn revision(payload: &[u8], object: &str) -> Revision {
    Revision::new(
        EncodingVersion::V1,
        ObjectType::new(object).expect("valid type"),
        payload.to_vec(),
    )
}

/// Verifies revision IDs match the shared core hash helper.
#[test]
fn revision_identity_is_content_addressed() {
    let value = revision(b"payload", "test.note");
    assert_eq!(
        value.id(),
        revision_id(EncodingVersion::V1, value.object_type(), value.payload())
    );
    assert_eq!(value.id(), revision(b"payload", "test.note").id());
    assert_ne!(value.id(), revision(b"other", "test.note").id());
    assert_ne!(value.id(), revision(b"payload", "test.other").id());
}

/// Verifies immutable revisions are deduplicated and returned by value.
#[test]
fn memory_store_is_idempotent_and_read_only() {
    let store = MemoryRevisionStore::new();
    let value = revision(b"payload", "test.note");
    let id = value.id();
    assert!(!store.contains(id).expect("contains"));
    assert_eq!(store.put(value.clone()).expect("put"), id);
    assert_eq!(store.put(value.clone()).expect("duplicate put"), id);
    assert!(store.contains(id).expect("contains"));
    assert_eq!(store.get(id).expect("get"), Some(value));
    assert_eq!(
        store
            .get(scriberig_core::RevisionId::new([9; 32]))
            .expect("missing"),
        None
    );
}
