//! Verifies strict revision identity validation and typed storage errors.

use scriberig_core::{EncodingVersion, ObjectType, RevisionId};
use scriberig_store::{Revision, RevisionStore, StoreError};

/// Provides an empty backend for absence behavior.
struct EmptyStore;

impl RevisionStore for EmptyStore {
    /// Rejects writes because this fixture is read-only.
    ///
    /// # Parameters
    /// - `_revision`: Revision that would be written.
    ///
    /// # Returns
    /// Never returns successfully.
    ///
    /// # Errors
    /// Always returns an invalid-operation error.
    fn put(&self, _revision: Revision) -> Result<RevisionId, StoreError> {
        Err(StoreError::InvalidOperation {
            operation: "empty fixture".to_owned(),
        })
    }

    /// Always reports that the revision is absent.
    ///
    /// # Parameters
    /// - `_id`: Revision identity to look up.
    ///
    /// # Returns
    /// `None` because this fixture contains no revisions.
    ///
    /// # Errors
    /// This fixture does not return backend errors.
    fn get(&self, _id: RevisionId) -> Result<Option<Revision>, StoreError> {
        Ok(None)
    }

    /// Always reports that the revision is absent.
    ///
    /// # Parameters
    /// - `_id`: Revision identity to check.
    ///
    /// # Returns
    /// `false` because this fixture contains no revisions.
    ///
    /// # Errors
    /// This fixture does not return backend errors.
    fn contains(&self, _id: RevisionId) -> Result<bool, StoreError> {
        Ok(false)
    }
}

/// Rejects an externally supplied identity that does not match canonical data.
#[test]
fn revision_parts_reject_identity_mismatch() {
    let object_type = ObjectType::new("test.item").expect("valid type");
    let actual = RevisionId::new([9; 32]);

    let error = Revision::from_parts(actual, EncodingVersion::V1, object_type, vec![1, 2, 3])
        .expect_err("mismatched identity must fail");

    assert!(matches!(error, StoreError::RevisionIdentityMismatch { .. }));
}

/// Exposes absence through the typed revision-store error taxonomy.
#[test]
fn missing_revision_is_a_typed_error() {
    let store = EmptyStore;
    let id = RevisionId::new([8; 32]);

    let error = store
        .get_required(id)
        .expect_err("missing revision must be reported");

    assert_eq!(error, StoreError::NotFound { id });
}

/// Rejects construction with an encoding version this build cannot interpret.
#[test]
fn revision_construction_rejects_unsupported_encoding_version() {
    let object_type = ObjectType::new("test.item").expect("valid type");

    let error = Revision::new(EncodingVersion::new(2), object_type, vec![1])
        .expect_err("unsupported version must fail");

    assert_eq!(
        error,
        StoreError::InvalidCanonicalEncoding(scriberig_core::EncodingError::UnsupportedVersion {
            version: 2
        })
    );
}
