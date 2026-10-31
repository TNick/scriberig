//! Defines backend-independent immutable revision storage behavior.

use crate::Revision;
use scriberig_core::{EncodingError, RevisionId, TypeId};
use std::collections::BTreeMap;

/// Reports a revision storage failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    /// A stable ID was found with different canonical fields.
    IntegrityCollision {
        /// Conflicting revision identity.
        id: RevisionId,
    },
    /// A canonical encoding version or envelope was not supported.
    InvalidCanonicalEncoding(EncodingError),
    /// The supplied identity does not match the canonical revision fields.
    RevisionIdentityMismatch {
        /// Identity recomputed from canonical fields.
        expected: RevisionId,
        /// Identity supplied by the caller.
        actual: RevisionId,
    },
    /// A requested revision was not retained.
    NotFound {
        /// Missing revision identity.
        id: RevisionId,
    },
    /// The requested storage operation is invalid in the current state.
    InvalidOperation {
        /// Machine-readable operation description.
        operation: String,
    },
    /// A synchronization primitive could not be read or written.
    LockPoisoned,
    /// A backend-specific durable storage failure.
    Backend(String),
}

/// Stores immutable revisions by their content identity.
pub trait RevisionStore {
    /// Inserts an immutable revision idempotently.
    ///
    /// - `revision`: Immutable revision to retain.
    ///
    /// The revision identity.
    ///
    /// Returns storage or integrity errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn put(&self, revision: Revision) -> Result<RevisionId, StoreError>;
    /// Reads an immutable revision by identity.
    ///
    /// - `id`: Revision identity.
    ///
    /// The retained revision, when present.
    ///
    /// Returns a storage synchronization error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn get(&self, id: RevisionId) -> Result<Option<Revision>, StoreError>;

    /// Reads a batch of immutable revisions.
    ///
    /// Backends may use a bulk query. The default preserves compatibility by
    /// reading revisions individually.
    ///
    /// # Parameters
    /// - `ids`: Immutable revision identities to load.
    ///
    /// # Returns
    /// Retained revisions keyed by identity; missing IDs are omitted.
    ///
    /// # Errors
    /// Returns a storage or integrity error.
    fn get_many(&self, ids: &[RevisionId]) -> Result<BTreeMap<RevisionId, Revision>, StoreError> {
        let mut revisions = BTreeMap::new();
        for id in ids {
            if let Some(revision) = self.get(*id)? {
                revisions.insert(*id, revision);
            }
        }
        Ok(revisions)
    }

    /// Reads only the declared type of an immutable revision.
    ///
    /// Backends may use a metadata-only index to avoid reading revision
    /// payloads. The default preserves compatibility by loading the revision.
    ///
    /// # Parameters
    /// - `id`: Immutable revision identity.
    ///
    /// # Returns
    /// The declared object type, or `None` when the revision is absent.
    ///
    /// # Errors
    /// Returns a storage or metadata validation error.
    fn get_object_type(&self, id: RevisionId) -> Result<Option<TypeId>, StoreError> {
        self.get(id)
            .map(|revision| revision.map(|value| value.object_type().clone()))
    }

    /// Reads declared types for a batch of revisions without loading payloads.
    ///
    /// Backends may use a bulk metadata query. The default preserves
    /// compatibility by reading metadata one revision at a time.
    ///
    /// # Parameters
    /// - `ids`: Immutable revision identities to inspect.
    ///
    /// # Returns
    /// Types for retained revisions, keyed by revision identity.
    ///
    /// # Errors
    /// Returns a storage or metadata validation error.
    fn get_object_types(
        &self,
        ids: &[RevisionId],
    ) -> Result<BTreeMap<RevisionId, TypeId>, StoreError> {
        let mut types = BTreeMap::new();
        for id in ids {
            if let Some(object_type) = self.get_object_type(*id)? {
                types.insert(*id, object_type);
            }
        }
        Ok(types)
    }

    /// Reads an immutable revision and reports absence as a typed error.
    ///
    /// - `id`: Revision identity.
    ///
    /// The retained revision.
    ///
    /// Returns `NotFound` when the revision is absent or propagates a backend
    /// error from `get`.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn get_required(&self, id: RevisionId) -> Result<Revision, StoreError> {
        self.get(id)?.ok_or(StoreError::NotFound { id })
    }
    /// Checks whether an immutable revision is retained.
    ///
    /// - `id`: Revision identity.
    ///
    /// Whether the revision is present.
    ///
    /// Returns a storage synchronization error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn contains(&self, id: RevisionId) -> Result<bool, StoreError>;
}
