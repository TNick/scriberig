//! Defines immutable canonical revision values.

use scriberig_core::{EncodingVersion, ObjectType, RevisionId, revision_id};

/// Stores the canonical fields that determine one immutable revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Revision {
    version: EncodingVersion,
    object_type: ObjectType,
    payload: Vec<u8>,
    id: RevisionId,
}

impl Revision {
    /// Computes an immutable revision identity from canonical fields.
    ///
    /// # Parameters
    /// - `version`: Canonical encoding version.
    /// - `object_type`: Semantic object type.
    /// - `payload`: Canonical payload bytes.
    ///
    /// # Returns
    /// An immutable revision with a computed content identity.
    #[must_use]
    pub fn new(version: EncodingVersion, object_type: ObjectType, payload: Vec<u8>) -> Self {
        let id = revision_id(version, &object_type, &payload);
        Self {
            version,
            object_type,
            payload,
            id,
        }
    }

    /// Returns the canonical encoding version.
    #[must_use]
    pub const fn version(&self) -> EncodingVersion {
        self.version
    }
    /// Returns the semantic object type.
    ///
    /// # Returns
    /// The immutable semantic object type.
    #[must_use]
    pub fn object_type(&self) -> &ObjectType {
        &self.object_type
    }
    /// Returns the canonical payload bytes.
    ///
    /// # Returns
    /// The canonical payload slice.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    /// Returns the computed immutable revision identity.
    #[must_use]
    pub const fn id(&self) -> RevisionId {
        self.id
    }
}
