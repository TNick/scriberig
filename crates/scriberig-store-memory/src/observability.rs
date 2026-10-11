//! Provides observability for the in-memory storage backend.

use scriberig_store::{IndexStatus, RootStatus, StoreObservability, StoreStats};

use crate::MemoryStore;

impl StoreObservability for MemoryStore {
    /// Returns counts from the in-memory authoritative collections.
    ///
    /// # Returns
    /// Available in-memory statistics.
    ///
    /// # Errors
    /// Returns an error when a collection lock is poisoned.
    fn stats(&self) -> Result<StoreStats, String> {
        let revisions = self
            .revisions
            .revisions
            .read()
            .map_err(|_| "revision lock poisoned".to_owned())?
            .len() as u64;
        let snapshots = self
            .snapshots
            .read()
            .map_err(|_| "snapshot lock poisoned".to_owned())?
            .len() as u64;
        let named_roots = self
            .roots
            .read()
            .map_err(|_| "root lock poisoned".to_owned())?
            .len() as u64;
        Ok(StoreStats {
            revisions: Some(revisions),
            snapshots: Some(snapshots),
            named_roots: Some(named_roots),
            ..StoreStats::default()
        })
    }

    /// Lists in-memory roots in deterministic order.
    ///
    /// # Returns
    /// Root status records ordered by root name.
    ///
    /// # Errors
    /// Returns an error when the root lock is poisoned.
    fn root_statuses(&self) -> Result<Vec<RootStatus>, String> {
        Ok(self
            .roots
            .read()
            .map_err(|_| "root lock poisoned".to_owned())?
            .iter()
            .map(|(name, snapshot)| RootStatus {
                name: name.clone(),
                snapshot: Some(*snapshot),
            })
            .collect())
    }

    /// Rebuilds the in-memory derived-index boundary.
    ///
    /// # Returns
    /// A consistent index status.
    ///
    /// # Errors
    /// This backend has no derived indexes and does not fail this operation.
    fn rebuild_indexes(&self) -> Result<IndexStatus, String> {
        Ok(IndexStatus { consistent: true })
    }
}
