//! Implements backend-neutral maintenance for the in-memory store.

use std::collections::BTreeSet;

use scriberig_store::{
    IndexStatus, IntegrityIssue, IntegrityReport, MaintenanceStore, ProtectedRoots,
    StoreOperationError, UnreachableObjects,
};

use crate::MemoryStore;

impl MaintenanceStore for MemoryStore {
    /// Verifies retained snapshots, revisions, and named roots.
    ///
    /// - `protected`: Stable snapshots for the verification.
    ///
    /// Integrity and reachability findings.
    ///
    /// Returns a lock-poisoning maintenance error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn integrity_check(
        &self,
        protected: &ProtectedRoots,
    ) -> Result<IntegrityReport, StoreOperationError> {
        let snapshots = self
            .snapshots
            .read()
            .map_err(|_| StoreOperationError::backend("snapshot lock poisoned"))?;
        let revisions = self
            .revisions
            .revisions
            .read()
            .map_err(|_| StoreOperationError::backend("revision lock poisoned"))?;
        let roots = self
            .roots
            .read()
            .map_err(|_| StoreOperationError::backend("root lock poisoned"))?;
        let mut report = IntegrityReport::default();
        for snapshot_id in &protected.snapshots {
            if !snapshots.contains_key(snapshot_id) {
                report
                    .corrupt
                    .push(IntegrityIssue::MissingSnapshot(*snapshot_id));
            }
        }
        for (name, snapshot) in roots.iter() {
            if !snapshots.contains_key(snapshot) {
                report
                    .corrupt
                    .push(IntegrityIssue::InvalidRoot(name.as_str().to_owned()));
            }
        }
        let mut reachable_snapshots: BTreeSet<_> = protected
            .snapshots
            .iter()
            .filter(|snapshot| snapshots.contains_key(snapshot))
            .copied()
            .collect();
        reachable_snapshots.insert(crate::Snapshot::empty().id());
        let mut reachable_revisions = BTreeSet::new();
        for snapshot in &reachable_snapshots {
            if let Some(snapshot) = snapshots.get(snapshot) {
                for (_, revision) in snapshot.iter() {
                    if revisions.contains_key(&revision) {
                        reachable_revisions.insert(revision);
                    }
                }
            }
        }
        report.unreachable = UnreachableObjects {
            snapshots: snapshots
                .keys()
                .filter(|snapshot| !reachable_snapshots.contains(snapshot))
                .copied()
                .collect(),
            revisions: revisions
                .keys()
                .filter(|revision| !reachable_revisions.contains(revision))
                .copied()
                .collect(),
            snapshot_nodes: BTreeSet::new(),
        };
        report.gc.candidates =
            report.unreachable.snapshots.len() as u64 + report.unreachable.revisions.len() as u64;
        Ok(report)
    }

    /// Runs a non-destructive maintenance report.
    ///
    /// - `protected`: Stable snapshots for the report.
    ///
    /// The integrity and reachability report.
    ///
    /// Returns a backend maintenance error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn gc_dry_run(
        &self,
        protected: &ProtectedRoots,
    ) -> Result<IntegrityReport, StoreOperationError> {
        self.integrity_check(protected)
    }

    /// Removes valid snapshots and revisions outside the protected set.
    ///
    /// - `protected`: Stable snapshots for the sweep.
    ///
    /// The pre-sweep integrity and reachability report.
    ///
    /// Returns a lock-poisoning or integrity error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn gc_sweep(&self, protected: &ProtectedRoots) -> Result<IntegrityReport, StoreOperationError> {
        let mut report = self.integrity_check(protected)?;
        if report.is_corrupt() {
            return Ok(report);
        }
        let mut snapshots = self
            .snapshots
            .write()
            .map_err(|_| StoreOperationError::backend("snapshot lock poisoned"))?;
        let mut revisions = self
            .revisions
            .revisions
            .write()
            .map_err(|_| StoreOperationError::backend("revision lock poisoned"))?;
        report.gc.deleted = report.gc.candidates;
        for snapshot in &report.unreachable.snapshots {
            snapshots.remove(snapshot);
        }
        for revision in &report.unreachable.revisions {
            revisions.remove(revision);
        }
        Ok(report)
    }

    /// Rebuilds the empty in-memory derived-index boundary.
    ///
    /// A consistent index status.
    ///
    /// This backend has no derived indexes and cannot fail this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn rebuild_indexes(&self) -> Result<IndexStatus, StoreOperationError> {
        Ok(IndexStatus { consistent: true })
    }
}
