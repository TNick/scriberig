//! Provides the in-memory implementation of the Scriberig storage contracts.

use std::collections::BTreeMap;
use std::sync::RwLock;

use scriberig_core::RevisionId;
use scriberig_store::{Revision, RevisionStore, StoreError};

/// Thread-safe in-memory immutable revision store.
pub struct MemoryRevisionStore {
    revisions: RwLock<BTreeMap<RevisionId, Revision>>,
}

impl Default for MemoryRevisionStore {
    /// Creates an empty store through the default constructor.
    ///
    /// # Returns
    /// An empty thread-safe store.
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryRevisionStore {
    /// Creates an empty in-memory revision store.
    ///
    /// # Returns
    /// An empty thread-safe store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            revisions: RwLock::new(BTreeMap::new()),
        }
    }
}

impl RevisionStore for MemoryRevisionStore {
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
    fn put(&self, revision: Revision) -> Result<RevisionId, StoreError> {
        let id = revision.id();
        let mut records = self
            .revisions
            .write()
            .map_err(|_| StoreError::LockPoisoned)?;
        if let Some(existing) = records.get(&id) {
            if existing != &revision {
                return Err(StoreError::IntegrityCollision { id });
            }
            return Ok(id);
        }
        records.insert(id, revision);
        Ok(id)
    }
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
    fn get(&self, id: RevisionId) -> Result<Option<Revision>, StoreError> {
        Ok(self
            .revisions
            .read()
            .map_err(|_| StoreError::LockPoisoned)?
            .get(&id)
            .cloned())
    }
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
    fn contains(&self, id: RevisionId) -> Result<bool, StoreError> {
        Ok(self
            .revisions
            .read()
            .map_err(|_| StoreError::LockPoisoned)?
            .contains_key(&id))
    }
}
