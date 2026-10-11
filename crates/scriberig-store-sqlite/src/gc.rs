//! Reachability inspection and garbage collection for SQLite storage.

use std::collections::BTreeSet;

use rusqlite::params;
use scriberig_core::{EncodingVersion, ObjectType, RevisionId, SnapshotId};
use scriberig_store::{
    IntegrityIssue, IntegrityReport, ProtectedRoots, Revision, UnreachableObjects,
};

use crate::SqliteStore;

impl SqliteStore {
    /// Computes immutable objects that are not protected by named roots.
    ///
    /// # Parameters
    /// - `protected`: Snapshot roots that must remain available.
    ///
    /// # Returns
    /// An integrity report and the objects eligible for collection.
    ///
    /// # Errors
    /// Returns the underlying SQLite error when inspection fails.
    pub fn gc_dry_run(&self, protected: &ProtectedRoots) -> rusqlite::Result<IntegrityReport> {
        self.inspect_gc(protected, false)
    }

    /// Deletes immutable objects that are not protected by the supplied roots.
    ///
    /// # Parameters
    /// - `protected`: Snapshot roots that must remain available.
    ///
    /// # Returns
    /// An integrity report describing removed candidates and detected corruption.
    ///
    /// # Errors
    /// Returns the underlying SQLite error when inspection or deletion fails.
    pub fn gc_sweep(&self, protected: &ProtectedRoots) -> rusqlite::Result<IntegrityReport> {
        self.inspect_gc(protected, true)
    }

    /// Inspects storage and optionally removes unreachable objects.
    ///
    /// # Parameters
    /// - `protected`: Snapshot roots that must remain available.
    /// - `sweep`: Whether to delete the reported candidates.
    ///
    /// # Returns
    /// A report of corruption and unreachable objects.
    ///
    /// # Errors
    /// Returns the underlying SQLite error when a query or transaction fails.
    #[allow(clippy::too_many_lines)]
    fn inspect_gc(
        &self,
        protected: &ProtectedRoots,
        sweep: bool,
    ) -> rusqlite::Result<IntegrityReport> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut report = IntegrityReport::default();
        let mut protected_revisions = BTreeSet::new();

        for snapshot_id in &protected.snapshots {
            let exists: bool = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM snapshots WHERE id=?)",
                params![snapshot_id.as_bytes().as_slice()],
                |row| row.get(0),
            )?;
            if !exists {
                report
                    .corrupt
                    .push(IntegrityIssue::MissingSnapshot(*snapshot_id));
                continue;
            }
            let mut entries =
                connection.prepare("SELECT revision FROM snapshot_entries WHERE snapshot_id=?")?;
            let revisions = entries
                .query_map(params![snapshot_id.as_bytes().as_slice()], |row| {
                    row.get::<_, Vec<u8>>(0)
                })?;
            for revision in revisions {
                let revision = revision?;
                if revision.len() == 32 {
                    let mut bytes = [0; 32];
                    bytes.copy_from_slice(&revision);
                    protected_revisions.insert(RevisionId::new(bytes));
                }
            }
        }

        let mut snapshots = BTreeSet::new();
        let mut rows = connection.prepare("SELECT id FROM snapshots")?;
        let ids = rows.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
        for id in ids {
            let id = id?;
            if id.len() != 32 {
                continue;
            }
            let mut bytes = [0; 32];
            bytes.copy_from_slice(&id);
            snapshots.insert(SnapshotId::new(bytes));
        }

        let mut revisions = BTreeSet::new();
        let mut rows =
            connection.prepare("SELECT id, version, object_type, payload FROM revisions")?;
        let ids = rows.query_map([], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                row.get::<_, u16>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
            ))
        })?;
        for id in ids {
            let (id, version, object_type, payload) = id?;
            if id.len() != 32 {
                continue;
            }
            let mut bytes = [0; 32];
            bytes.copy_from_slice(&id);
            let revision_id = RevisionId::new(bytes);
            revisions.insert(revision_id);
            let valid = ObjectType::new(object_type).is_ok_and(|object_type| {
                Revision::new(EncodingVersion::new(version), object_type, payload).id()
                    == revision_id
            });
            if !valid {
                report
                    .corrupt
                    .push(IntegrityIssue::RevisionMismatch(revision_id));
            }
        }

        report.unreachable = UnreachableObjects {
            snapshots: snapshots
                .difference(&protected.snapshots)
                .copied()
                .collect(),
            revisions: revisions
                .difference(&protected_revisions)
                .copied()
                .collect(),
        };

        if sweep {
            let transaction = connection.unchecked_transaction()?;
            for snapshot_id in &report.unreachable.snapshots {
                transaction.execute(
                    "DELETE FROM snapshot_entries WHERE snapshot_id=?",
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
            transaction.commit()?;
        }

        Ok(report)
    }
}
