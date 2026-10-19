//! Defines the atomic in-memory named-root backend.
use scriberig_core::{RootName, SnapshotId};
use scriberig_store::{CasOutcome, RootError, RootStore};
use std::collections::BTreeMap;
use std::sync::RwLock;

/// Atomic in-memory named-root store.
pub struct MemoryRootStore {
    roots: RwLock<BTreeMap<RootName, SnapshotId>>,
}

impl Default for MemoryRootStore {
    ///
    /// The requested root result.
    /// Creates an empty root store.
    /// # Returns
    /// The operation result.
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryRootStore {
    ///
    /// The requested root result.
    /// Creates an empty named-root store.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new() -> Self {
        Self {
            roots: RwLock::new(BTreeMap::new()),
        }
    }
}

impl RootStore for MemoryRootStore {
    /// - `name`: Operation input.
    ///
    /// The requested root result.
    /// Reads a named root.
    /// # Returns
    /// The operation result.
    fn read_root(&self, name: &RootName) -> Option<SnapshotId> {
        self.roots.read().ok()?.get(name).copied()
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
    /// - `name`: Operation input.
    /// - `expected`: Operation input.
    /// - `new`: Operation input.
    ///
    /// The requested root result.
    ///
    /// Returns a root backend error.
    /// Promotes a root using an atomic compare-and-set.
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
        let mut roots = self
            .roots
            .write()
            .map_err(|_| RootError::Backend("lock poisoned".to_owned()))?;
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
    /// - `name`: Operation input.
    /// - `expected`: Operation input.
    /// - `new`: Operation input.
    ///
    /// The requested root result.
    ///
    /// Returns a root backend error.
    /// Performs an atomic compare-and-swap with an explicit stale outcome.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_swap(
        &self,
        name: RootName,
        expected: Option<SnapshotId>,
        new: SnapshotId,
    ) -> Result<CasOutcome, RootError> {
        let mut roots = self
            .roots
            .write()
            .map_err(|_| RootError::Backend("lock poisoned".to_owned()))?;
        let actual = roots.get(&name).copied();
        if actual != expected {
            return Ok(CasOutcome::Stale { actual });
        }
        roots.insert(name, new);
        Ok(CasOutcome::Updated)
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryRootStore;
    use scriberig_core::{RootName, SnapshotId};
    use scriberig_store::RootStore;

    /// Confirms named roots reject stale promotion without mutation.
    #[test]
    fn roots_compare_and_set() {
        let store = MemoryRootStore::new();
        let name = RootName::new("main").expect("root");
        let base = SnapshotId::new([1; 32]);
        let next = SnapshotId::new([2; 32]);
        store
            .compare_and_set_root(name.clone(), None, base)
            .expect("initial promotion");
        assert!(
            store
                .compare_and_set_root(name.clone(), None, next)
                .is_err()
        );
        assert_eq!(store.read_root(&name), Some(base));
    }

    /// Confirms unconditional setting replaces a concurrent root value.
    #[test]
    fn set_root_replaces_existing_value() {
        let store = MemoryRootStore::new();
        let name = RootName::new("main").expect("root");
        let first = SnapshotId::new([1; 32]);
        let second = SnapshotId::new([2; 32]);

        store.set_root(name.clone(), first).expect("first root");
        store.set_root(name.clone(), second).expect("second root");

        assert_eq!(store.read_root(&name), Some(second));
    }
}
