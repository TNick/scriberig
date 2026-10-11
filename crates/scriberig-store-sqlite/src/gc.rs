//! Performs conservative SQLite mark-and-sweep maintenance.

use rusqlite::params;
use scriberig_store::{IntegrityReport, ProtectedRoots};

use crate::SqliteStore;

impl SqliteStore {
    /// Computes immutable objects that are not protected by a stable root set.
    ///
    /// # Parameters
    /// - `protected`: Explicit snapshot roots captured for this maintenance run.
    ///
    /// # Returns
    /// Integrity findings and valid unreachable candidates.
    ///
    /// # Errors
    /// Returns an error when canonical storage cannot be inspected.
    pub fn gc_dry_run(&self, protected: &ProtectedRoots) -> rusqlite::Result<IntegrityReport> {
        self.integrity_check(protected)
    }

    /// Deletes valid immutable objects proven unreachable from protected roots.
    ///
    /// # Parameters
    /// - `protected`: Explicit snapshot roots captured for this maintenance run.
    ///
    /// # Returns
    /// The pre-sweep integrity report and candidate set.
    ///
    /// # Errors
    /// Returns an error when inspection or the sweep transaction fails.
    pub fn gc_sweep(&self, protected: &ProtectedRoots) -> rusqlite::Result<IntegrityReport> {
        let report = self.integrity_check(protected)?;
        if report.is_corrupt() {
            return Ok(report);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let transaction = connection.unchecked_transaction()?;
        for snapshot_id in &report.unreachable.snapshots {
            transaction.execute(
                "DELETE FROM snapshot_entries WHERE snapshot_id=?",
                params![snapshot_id.as_bytes().as_slice()],
            )?;
            transaction.execute(
                "DELETE FROM snapshot_node_roots WHERE snapshot_id=?",
                params![snapshot_id.as_bytes().as_slice()],
            )?;
            transaction.execute(
                "DELETE FROM snapshots WHERE id=?",
                params![snapshot_id.as_bytes().as_slice()],
            )?;
        }
        for revision_id in &report.unreachable.revisions {
            transaction.execute(
                "DELETE FROM revisions WHERE id=?",
                params![revision_id.as_bytes().as_slice()],
            )?;
        }
        for node_id in &report.unreachable.snapshot_nodes {
            transaction.execute(
                "DELETE FROM snapshot_nodes WHERE id=?",
                params![node_id.as_bytes().as_slice()],
            )?;
        }
        transaction.commit()?;
        Ok(report)
    }
}
