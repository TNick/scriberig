//! Provides the in-memory implementation of the Scriberig storage contracts.

mod observability;
mod roots;
pub use roots::MemoryRootStore;

use std::collections::BTreeMap;
use std::sync::RwLock;

use scriberig_core::{EntityId, RevisionId, RootName, SnapshotId};
use scriberig_store::{
    Revision, RevisionStore, RootError, RootStore, Snapshot, SnapshotChange, SnapshotError,
    SnapshotStore, StoreError,
};

/// Thread-safe in-memory immutable revision store.
pub struct MemoryRevisionStore {
    revisions: RwLock<BTreeMap<RevisionId, Revision>>,
}

/// Thread-safe in-memory revisions, snapshots, and named roots.
pub struct MemoryStore {
    revisions: MemoryRevisionStore,
    snapshots: RwLock<BTreeMap<SnapshotId, Snapshot>>,
    roots: RwLock<BTreeMap<RootName, SnapshotId>>,
}

impl Default for MemoryStore {
    ///
    /// # Returns
    /// The requested operation result.
    /// Creates an empty memory backend.
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    ///
    /// # Returns
    /// The requested operation result.
    /// Creates an empty memory backend with a deterministic empty snapshot.
    #[must_use]
    pub fn new() -> Self {
        let empty = Snapshot::from_entries(BTreeMap::new());
        let id = empty.id();
        let mut snapshots = BTreeMap::new();
        snapshots.insert(id, empty);
        Self {
            revisions: MemoryRevisionStore::new(),
            snapshots: RwLock::new(snapshots),
            roots: RwLock::new(BTreeMap::new()),
        }
    }

    ///
    /// # Returns
    /// The requested operation result.
    /// Returns the deterministic empty snapshot identity.
    #[must_use]
    pub fn empty_snapshot(&self) -> SnapshotId {
        Snapshot::from_entries(BTreeMap::new()).id()
    }

    ///
    /// # Returns
    /// The requested operation result.
    /// Returns the immutable revision backend.
    #[must_use]
    pub const fn revisions(&self) -> &MemoryRevisionStore {
        &self.revisions
    }

    /// # Parameters
    /// - `id`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    fn snapshot(&self, id: SnapshotId) -> Result<Snapshot, SnapshotError> {
        self.snapshots
            .read()
            .map_err(|_| SnapshotError::UnknownSnapshot(id))?
            .get(&id)
            .cloned()
            .ok_or(SnapshotError::UnknownSnapshot(id))
    }
}

impl SnapshotStore for MemoryStore {
    /// # Parameters
    /// - `snapshot`: Relevant operation input.
    /// - `entity`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Resolves an entity from a retained snapshot.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.resolve(entity))
    }

    /// # Parameters
    /// - `base`: Relevant operation input.
    /// - `changes`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Applies changes and retains the resulting immutable snapshot.
    fn apply_changes(
        &self,
        base: SnapshotId,
        changes: &[SnapshotChange],
    ) -> Result<SnapshotId, SnapshotError> {
        let next = self.snapshot(base)?.apply(changes.iter().copied());
        let id = next.id();
        self.snapshots
            .write()
            .map_err(|_| SnapshotError::UnknownSnapshot(id))?
            .entry(id)
            .or_insert(next);
        Ok(id)
    }

    /// # Parameters
    /// - `snapshot`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Iterates a retained snapshot in entity order.
    fn iter(&self, snapshot: SnapshotId) -> Result<Vec<(EntityId, RevisionId)>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.iter().collect())
    }

    /// # Parameters
    /// - `from`: Relevant operation input.
    /// - `to`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Computes deterministic changes between retained snapshots.
    fn diff_snapshots(
        &self,
        from: SnapshotId,
        to: SnapshotId,
    ) -> Result<Vec<SnapshotChange>, SnapshotError> {
        Ok(self.snapshot(from)?.diff(&self.snapshot(to)?))
    }
}

impl RootStore for MemoryStore {
    /// # Parameters
    /// - `name`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    /// Reads a named root atomically.
    fn read_root(&self, name: &RootName) -> Option<SnapshotId> {
        self.roots.read().ok()?.get(name).copied()
    }

    /// # Parameters
    /// - `name`: Relevant operation input.
    /// - `expected`: Relevant operation input.
    /// - `new`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
    /// Promotes a named root using an atomic compare-and-set.
    fn compare_and_set_root(
        &self,
        name: RootName,
        expected: Option<SnapshotId>,
        new: SnapshotId,
    ) -> Result<(), RootError> {
        let mut roots = self.roots.write().map_err(|_| RootError::Conflict {
            name: name.clone(),
            expected,
            actual: None,
        })?;
        let actual = roots.get(&name).copied();
        if actual != expected {
            return Err(RootError::Conflict {
                name,
                expected,
                actual,
            });
        }
        roots.insert(name, new);
        Ok(())
    }
}

impl Default for MemoryRevisionStore {
    ///
    /// # Returns
    /// The requested operation result.
    /// Creates an empty store through the default constructor.
    ///
    /// # Returns
    /// An empty thread-safe store.
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryRevisionStore {
    ///
    /// # Returns
    /// The requested operation result.
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
    /// # Parameters
    /// - `revision`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
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
    /// # Parameters
    /// - `id`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
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
    /// # Parameters
    /// - `id`: Relevant operation input.
    ///
    /// # Returns
    /// The requested operation result.
    ///
    /// # Errors
    /// Returns an error when the operation cannot be completed.
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

impl RevisionStore for MemoryStore {
    /// Inserts a revision into the shared memory revision collection.
    ///
    /// # Parameters
    /// - `revision`: Immutable revision to retain.
    ///
    /// # Returns
    /// The revision identity.
    ///
    /// # Errors
    /// Returns a storage error when the revision lock is poisoned.
    fn put(&self, revision: Revision) -> Result<RevisionId, StoreError> {
        self.revisions.put(revision)
    }

    /// Reads a revision from the shared memory revision collection.
    ///
    /// # Parameters
    /// - `id`: Revision identity.
    ///
    /// # Returns
    /// The retained revision, when present.
    ///
    /// # Errors
    /// Returns a storage error when the revision lock is poisoned.
    fn get(&self, id: RevisionId) -> Result<Option<Revision>, StoreError> {
        self.revisions.get(id)
    }

    /// Checks whether a revision is retained in memory.
    ///
    /// # Parameters
    /// - `id`: Revision identity.
    ///
    /// # Returns
    /// Whether the revision is present.
    ///
    /// # Errors
    /// Returns a storage error when the revision lock is poisoned.
    fn contains(&self, id: RevisionId) -> Result<bool, StoreError> {
        self.revisions.contains(id)
    }
}

#[cfg(test)]
mod tests;
