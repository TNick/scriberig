//! Defines backend-independent immutable revision storage behavior.

use crate::Revision;
use scriberig_core::RevisionId;

/// Reports a revision storage failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreError {
    /// A stable ID was found with different canonical fields.
    IntegrityCollision {
        /// Conflicting revision identity.
        id: RevisionId,
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
    /// # Parameters
    /// - `revision`: Immutable revision to retain.
    ///
    /// # Returns
    /// The revision identity.
    ///
    /// # Errors
    /// Returns storage or integrity errors.
    fn put(&self, revision: Revision) -> Result<RevisionId, StoreError>;
    /// Reads an immutable revision by identity.
    ///
    /// # Parameters
    /// - `id`: Revision identity.
    ///
    /// # Returns
    /// The retained revision, when present.
    ///
    /// # Errors
    /// Returns a storage synchronization error.
    fn get(&self, id: RevisionId) -> Result<Option<Revision>, StoreError>;
    /// Checks whether an immutable revision is retained.
    ///
    /// # Parameters
    /// - `id`: Revision identity.
    ///
    /// # Returns
    /// Whether the revision is present.
    ///
    /// # Errors
    /// Returns a storage synchronization error.
    fn contains(&self, id: RevisionId) -> Result<bool, StoreError>;
}
