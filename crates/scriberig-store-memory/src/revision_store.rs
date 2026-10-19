//! Implements in-memory revision storage.

use super::{
    BTreeMap, MemoryRevisionStore, Revision, RevisionId, RevisionStore, RwLock, StoreError,
};

impl Default for MemoryRevisionStore {
    ///
    /// The requested operation result.
    /// Creates an empty store through the default constructor.
    ///
    /// An empty thread-safe store.
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryRevisionStore {
    ///
    /// The requested operation result.
    /// Creates an empty in-memory revision store.
    ///
    /// An empty thread-safe store.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new() -> Self {
        Self {
            revisions: RwLock::new(BTreeMap::new()),
        }
    }
}

impl RevisionStore for MemoryRevisionStore {
    /// - `revision`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
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
    /// - `id`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
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
    fn get(&self, id: RevisionId) -> Result<Option<Revision>, StoreError> {
        Ok(self
            .revisions
            .read()
            .map_err(|_| StoreError::LockPoisoned)?
            .get(&id)
            .cloned())
    }
    /// - `id`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
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
    fn contains(&self, id: RevisionId) -> Result<bool, StoreError> {
        Ok(self
            .revisions
            .read()
            .map_err(|_| StoreError::LockPoisoned)?
            .contains_key(&id))
    }
}
