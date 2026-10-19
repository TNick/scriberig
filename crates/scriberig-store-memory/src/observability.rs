//! Provides observability for the in-memory storage backend.

use scriberig_store::{
    IndexStatus, MaintenanceStore, ProtectedRoots, RootStatus, StoreObservability,
    StoreOperationError, StoreStats,
};

use crate::MemoryStore;

impl StoreObservability for MemoryStore {
    /// Returns counts from the in-memory authoritative collections.
    ///
    /// Available in-memory statistics.
    ///
    /// Returns an error when a collection lock is poisoned.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn stats(&self) -> Result<StoreStats, StoreOperationError> {
        let revisions = self
            .revisions
            .revisions
            .read()
            .map_err(|_| StoreOperationError::backend("revision lock poisoned"))?
            .len() as u64;
        let snapshots = self
            .snapshots
            .read()
            .map_err(|_| StoreOperationError::backend("snapshot lock poisoned"))?
            .len() as u64;
        let named_roots = self
            .roots
            .read()
            .map_err(|_| StoreOperationError::backend("root lock poisoned"))?
            .len() as u64;
        Ok(StoreStats {
            revisions: Some(revisions),
            snapshots: Some(snapshots),
            named_roots: Some(named_roots),
            ..StoreStats::default()
        })
    }

    /// Returns counts annotated with memory reachability.
    ///
    /// - `protected`: Snapshot roots used for reachability counts.
    ///
    /// Authoritative counts and marked-object counts.
    ///
    /// Returns a maintenance error when a collection lock is poisoned.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn stats_for(&self, protected: &ProtectedRoots) -> Result<StoreStats, StoreOperationError> {
        let mut stats = self.stats()?;
        let report = MaintenanceStore::integrity_check(self, protected)?;
        let total = stats.revisions.unwrap_or(0) + stats.snapshots.unwrap_or(0);
        let unreachable =
            (report.unreachable.revisions.len() + report.unreachable.snapshots.len()) as u64;
        stats.reachable_objects = Some(total.saturating_sub(unreachable));
        stats.unreachable_objects = Some(unreachable);
        Ok(stats)
    }

    /// Lists in-memory roots in deterministic order.
    ///
    /// Root status records ordered by root name.
    ///
    /// Returns an error when the root lock is poisoned.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn root_statuses(&self) -> Result<Vec<RootStatus>, StoreOperationError> {
        Ok(self
            .roots
            .read()
            .map_err(|_| StoreOperationError::backend("root lock poisoned"))?
            .iter()
            .map(|(name, snapshot)| RootStatus {
                name: name.clone(),
                snapshot: Some(*snapshot),
            })
            .collect())
    }

    /// Rebuilds the in-memory derived-index boundary.
    ///
    /// A consistent index status.
    ///
    /// This backend has no derived indexes and does not fail this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn rebuild_indexes(&self) -> Result<IndexStatus, StoreOperationError> {
        Ok(IndexStatus { consistent: true })
    }
}
