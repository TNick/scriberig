//! Implements backend-neutral maintenance for the in-memory store.

use std::collections::BTreeSet;

use scriberig_store::{
    IndexStatus, IntegrityIssue, IntegrityReport, MaintenanceStore, ProtectedRoots,
    UnreachableObjects,
};

use crate::MemoryStore;

impl MaintenanceStore for MemoryStore {
    /// Verifies retained snapshots, revisions, and named roots.
    ///
    /// # Parameters
    /// - `protected`: Stable snapshots for the verification.
    ///
    /// # Returns
    /// Integrity and reachability findings.
    ///
    /// # Errors
    /// Returns a lock-poisoning maintenance error.
    fn integrity_check(&self, protected: &ProtectedRoots) -> Result<IntegrityReport, String> {
        let snapshots = self
            .snapshots
            .read()
            .map_err(|_| "snapshot lock poisoned".to_owned())?;
        let revisions = self
            .revisions
            .revisions
            .read()
            .map_err(|_| "revision lock poisoned".to_owned())?;
        let roots = self
            .roots
            .read()
            .map_err(|_| "root lock poisoned".to_owned())?;
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
        Ok(report)
    }

    /// Runs a non-destructive maintenance report.
    ///
    /// # Parameters
    /// - `protected`: Stable snapshots for the report.
    ///
    /// # Returns
    /// The integrity and reachability report.
    ///
    /// # Errors
    /// Returns a backend maintenance error.
    fn gc_dry_run(&self, protected: &ProtectedRoots) -> Result<IntegrityReport, String> {
        self.integrity_check(protected)
    }

    /// Removes valid snapshots and revisions outside the protected set.
    ///
    /// # Parameters
    /// - `protected`: Stable snapshots for the sweep.
    ///
    /// # Returns
    /// The pre-sweep integrity and reachability report.
    ///
    /// # Errors
    /// Returns a lock-poisoning or integrity error.
    fn gc_sweep(&self, protected: &ProtectedRoots) -> Result<IntegrityReport, String> {
        let report = self.integrity_check(protected)?;
        if report.is_corrupt() {
            return Ok(report);
        }
        let mut snapshots = self
            .snapshots
            .write()
            .map_err(|_| "snapshot lock poisoned".to_owned())?;
        let mut revisions = self
            .revisions
            .revisions
            .write()
            .map_err(|_| "revision lock poisoned".to_owned())?;
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
    /// # Returns
    /// A consistent index status.
    ///
    /// # Errors
    /// This backend has no derived indexes and cannot fail this operation.
    fn rebuild_indexes(&self) -> Result<IndexStatus, String> {
        Ok(IndexStatus { consistent: true })
    }
}
