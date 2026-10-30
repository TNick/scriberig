//! Provides the in-memory implementation of the Scriberig storage contracts.

mod maintenance;
mod observability;
mod revision_store;
mod roots;
pub use roots::MemoryRootStore;

use std::collections::{BTreeMap, BTreeSet};
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
    parents: RwLock<BTreeMap<SnapshotId, BTreeSet<SnapshotId>>>,
    roots: RwLock<BTreeMap<RootName, SnapshotId>>,
}

impl Default for MemoryStore {
    ///
    /// The requested operation result.
    /// Creates an empty memory backend.
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore {
    ///
    /// The requested operation result.
    /// Creates an empty memory backend with a deterministic empty snapshot.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new() -> Self {
        let empty = Snapshot::from_entries(BTreeMap::new());
        let id = empty.id();
        let mut snapshots = BTreeMap::new();
        snapshots.insert(id, empty);
        Self {
            revisions: MemoryRevisionStore::new(),
            snapshots: RwLock::new(snapshots),
            parents: RwLock::new(BTreeMap::new()),
            roots: RwLock::new(BTreeMap::new()),
        }
    }

    ///
    /// The requested operation result.
    /// Returns the deterministic empty snapshot identity.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn empty_snapshot(&self) -> SnapshotId {
        Snapshot::from_entries(BTreeMap::new()).id()
    }

    ///
    /// The requested operation result.
    /// Returns the immutable revision backend.
    #[must_use]
    pub const fn revisions(&self) -> &MemoryRevisionStore {
        &self.revisions
    }

    /// - `id`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
    /// - `snapshot`: Relevant operation input.
    /// - `entity`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Resolves an entity from a retained snapshot.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn resolve(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<Option<RevisionId>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.resolve(entity))
    }

    /// - `base`: Relevant operation input.
    /// - `changes`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Applies changes and retains the resulting immutable snapshot.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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
        if id != base {
            self.parents
                .write()
                .map_err(|_| SnapshotError::UnknownSnapshot(id))?
                .entry(id)
                .or_default()
                .insert(base);
        }
        Ok(id)
    }

    /// - `snapshot`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Iterates a retained snapshot in entity order.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn iter(&self, snapshot: SnapshotId) -> Result<Vec<(EntityId, RevisionId)>, SnapshotError> {
        Ok(self.snapshot(snapshot)?.iter().collect())
    }

    /// - `from`: Relevant operation input.
    /// - `to`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Computes deterministic changes between retained snapshots.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn diff_snapshots(
        &self,
        from: SnapshotId,
        to: SnapshotId,
    ) -> Result<Vec<SnapshotChange>, SnapshotError> {
        Ok(self.snapshot(from)?.diff_changes(&self.snapshot(to)?))
    }

    /// Returns direct immutable predecessors retained for one snapshot.
    ///
    /// - `snapshot`: Snapshot whose direct parents are requested.
    ///
    /// Direct parents in deterministic order.
    ///
    /// Returns an error when the snapshot is unavailable or the store lock is poisoned.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn parents(&self, snapshot: SnapshotId) -> Result<Vec<SnapshotId>, SnapshotError> {
        let _ = self.snapshot(snapshot)?;
        Ok(self
            .parents
            .read()
            .map_err(|_| SnapshotError::UnknownSnapshot(snapshot))?
            .get(&snapshot)
            .map(|parents| parents.iter().copied().collect())
            .unwrap_or_default())
    }
}

impl RootStore for MemoryStore {
    /// - `name`: Relevant operation input.
    ///
    /// The requested operation result.
    /// Reads a named root atomically.
    /// # Returns
    /// The operation result.
    fn read_root(&self, name: &RootName) -> Option<SnapshotId> {
        self.roots.read().ok()?.get(name).copied()
    }

    /// Lists the roots in deterministic name order.
    ///
    /// # Returns
    /// Every currently registered root paired with its snapshot.
    ///
    /// # Errors
    /// Returns an error when the root map cannot be read.
    fn list_roots(&self) -> Result<Vec<(RootName, SnapshotId)>, RootError> {
        self.roots
            .read()
            .map(|roots| {
                roots
                    .iter()
                    .map(|(name, snapshot)| (name.clone(), *snapshot))
                    .collect()
            })
            .map_err(|_| RootError::Backend("lock poisoned".to_owned()))
    }

    /// Unconditionally points a named root at an immutable snapshot.
    /// - `name`: Argument named `name`.
    /// - `snapshot`: Argument named `snapshot`.
    ///
    ///
    /// Returns an error when the operation cannot complete.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_root(&self, name: RootName, snapshot: SnapshotId) -> Result<(), RootError> {
        self.roots
            .write()
            .map_err(|_| RootError::Backend("lock poisoned".to_owned()))?
            .insert(name, snapshot);
        Ok(())
    }

    /// - `name`: Relevant operation input.
    /// - `expected`: Relevant operation input.
    /// - `new`: Relevant operation input.
    ///
    /// The requested operation result.
    ///
    /// Returns an error when the operation cannot be completed.
    /// Promotes a named root using an atomic compare-and-set.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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

impl RevisionStore for MemoryStore {
    /// Inserts a revision into the shared memory revision collection.
    ///
    /// - `revision`: Immutable revision to retain.
    ///
    /// The revision identity.
    ///
    /// Returns a storage error when the revision lock is poisoned.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn put(&self, revision: Revision) -> Result<RevisionId, StoreError> {
        self.revisions.put(revision)
    }

    /// Reads a revision from the shared memory revision collection.
    ///
    /// - `id`: Revision identity.
    ///
    /// The retained revision, when present.
    ///
    /// Returns a storage error when the revision lock is poisoned.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn get(&self, id: RevisionId) -> Result<Option<Revision>, StoreError> {
        self.revisions.get(id)
    }

    /// Checks whether a revision is retained in memory.
    ///
    /// - `id`: Revision identity.
    ///
    /// Whether the revision is present.
    ///
    /// Returns a storage error when the revision lock is poisoned.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn contains(&self, id: RevisionId) -> Result<bool, StoreError> {
        self.revisions.contains(id)
    }
}

#[cfg(test)]
mod tests;
