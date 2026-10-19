//! Defines immutable canonical revision values.

use crate::revision_store::StoreError;
use scriberig_core::{EncodingVersion, RevisionId, TypeId, revision_id};

/// Stores the canonical fields that determine one immutable revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Revision {
    version: EncodingVersion,
    object_type: TypeId,
    payload: Vec<u8>,
    id: RevisionId,
}

impl Revision {
    /// Validates an externally supplied revision identity against its fields.
    ///
    /// - `id`: Claimed immutable revision identity.
    /// - `version`: Canonical encoding version.
    /// - `object_type`: Semantic object type.
    /// - `payload`: Canonical payload bytes.
    ///
    /// A revision record when the claimed identity matches.
    ///
    /// Returns an identity or encoding error when validation fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn from_parts(
        id: RevisionId,
        version: EncodingVersion,
        object_type: TypeId,
        payload: Vec<u8>,
    ) -> Result<Self, StoreError> {
        EncodingVersion::try_new(version.value()).map_err(StoreError::InvalidCanonicalEncoding)?;
        let expected = revision_id(version, &object_type, &payload)
            .map_err(StoreError::InvalidCanonicalEncoding)?;
        if expected != id {
            return Err(StoreError::RevisionIdentityMismatch {
                expected,
                actual: id,
            });
        }
        Ok(Self {
            version,
            object_type,
            payload,
            id,
        })
    }

    /// Computes an immutable revision identity from canonical fields.
    ///
    /// - `version`: Canonical encoding version.
    /// - `object_type`: Semantic object type.
    /// - `payload`: Canonical payload bytes.
    ///
    /// An immutable revision with a computed content identity.
    ///
    /// Returns an encoding error when the version is unsupported.
    ///
    /// Returns an encoding error when `version` is unsupported.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn new(
        version: EncodingVersion,
        object_type: TypeId,
        payload: Vec<u8>,
    ) -> Result<Self, StoreError> {
        let id = revision_id(version, &object_type, &payload)
            .map_err(StoreError::InvalidCanonicalEncoding)?;
        Ok(Self {
            version,
            object_type,
            payload,
            id,
        })
    }

    /// Returns the canonical encoding version.
    #[must_use]
    pub const fn version(&self) -> EncodingVersion {
        self.version
    }
    /// Returns the semantic object type.
    ///
    /// The immutable semantic object type.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn object_type(&self) -> &TypeId {
        &self.object_type
    }
    /// Returns the canonical payload bytes.
    ///
    /// The canonical payload slice.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    /// Returns the computed immutable revision identity.
    #[must_use]
    pub const fn id(&self) -> RevisionId {
        self.id
    }
}
