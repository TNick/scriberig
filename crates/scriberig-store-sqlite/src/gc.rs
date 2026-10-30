//! Performs conservative SQLite mark-and-sweep maintenance.

use rusqlite::params;
use scriberig_store::{IntegrityReport, ProtectedRoots};

use crate::{SqliteStore, integrity::integrity_check_connection};

impl SqliteStore {
    /// Computes immutable objects that are not protected by a stable root set.
    ///
    /// - `protected`: Explicit snapshot roots captured for this maintenance run.
    ///
    /// Integrity findings and valid unreachable candidates.
    ///
    /// Returns an error when canonical storage cannot be inspected.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn gc_dry_run(&self, protected: &ProtectedRoots) -> rusqlite::Result<IntegrityReport> {
        self.integrity_check(protected)
    }

    /// Deletes valid immutable objects proven unreachable from protected roots.
    ///
    /// - `protected`: Explicit snapshot roots captured for this maintenance run.
    ///
    /// The pre-sweep integrity report and candidate set.
    ///
    /// Returns an error when inspection or the sweep transaction fails.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn gc_sweep(&self, protected: &ProtectedRoots) -> rusqlite::Result<IntegrityReport> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let transaction = connection.unchecked_transaction()?;
        let mut report = integrity_check_connection(&transaction, protected)?;
        if report.is_corrupt() {
            return Ok(report);
        }
        for snapshot_id in &report.unreachable.snapshots {
            transaction.execute(
                "DELETE FROM snapshot_relations WHERE snapshot_id=?",
                params![snapshot_id.as_bytes().as_slice()],
            )?;
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
        report.gc.deleted = report.gc.candidates;
        transaction.commit()?;
        Ok(report)
    }
}
