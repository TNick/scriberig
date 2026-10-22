//! Provides atomic semantic snapshot promotion operations.

use rusqlite::Transaction;
use scriberig_core::{RootName, SnapshotId};
use scriberig_store::{Revision, SnapshotChange};

use crate::{PromotionError, SqliteStore, revision, roots, snapshot};

impl SqliteStore {
    /// Promotes immutable revisions and their named root atomically.
    ///
    /// # Parameters
    /// - `name`: Named root to promote.
    /// - `expected`: Root value required for promotion.
    /// - `base`: Existing snapshot to change.
    /// - `revisions`: New immutable revisions to insert idempotently.
    /// - `changes`: Mapping changes to apply to the successor snapshot.
    ///
    /// # Returns
    /// The promoted immutable snapshot identity.
    ///
    /// # Errors
    /// Returns a snapshot, missing-revision, stale-root, or transaction error.
    pub fn apply_revisions_and_promote(
        &self,
        name: RootName,
        expected: Option<SnapshotId>,
        base: SnapshotId,
        revisions: &[Revision],
        changes: &[SnapshotChange],
    ) -> Result<SnapshotId, PromotionError> {
        self.apply_revisions_and_promote_with(name, expected, base, revisions, changes, |_| Ok(()))
    }

    /// Promotes a semantic snapshot and performs a related write in one transaction.
    ///
    /// The callback runs after the new root is written and before commit. It
    /// can update application-owned tables in this same SQLite database while
    /// keeping plan semantics outside the generic store crate.
    ///
    /// # Parameters
    /// - `name`: Named root to promote.
    /// - `expected`: Root value required for promotion.
    /// - `base`: Existing snapshot to change.
    /// - `revisions`: New immutable revisions to insert idempotently.
    /// - `changes`: Mapping changes to apply to the successor snapshot.
    /// - `related_write`: Additional write using the active SQLite transaction.
    ///
    /// # Returns
    /// The promoted immutable snapshot identity.
    ///
    /// # Errors
    /// Returns a snapshot, missing-revision, stale-root, callback, or transaction error.
    pub fn apply_revisions_and_promote_with(
        &self,
        name: RootName,
        expected: Option<SnapshotId>,
        base: SnapshotId,
        revisions: &[Revision],
        changes: &[SnapshotChange],
        related_write: impl FnOnce(&Transaction<'_>) -> Result<(), String>,
    ) -> Result<SnapshotId, PromotionError> {
        let source = self.snapshot(base).map_err(PromotionError::Snapshot)?;
        let next = source.apply(changes.iter().copied());
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| PromotionError::Backend("lock poisoned".to_owned()))?;
        let transaction = connection
            .transaction()
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        for revision in revisions {
            revision::insert_in_transaction(&transaction, revision)
                .map_err(|error| PromotionError::Backend(error.to_string()))?;
        }
        for change in changes {
            if let SnapshotChange::Set(_, revision) = change {
                let present: bool = transaction
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM revisions WHERE id=?)",
                        rusqlite::params![revision.as_bytes().as_slice()],
                        |row| row.get(0),
                    )
                    .map_err(|error| PromotionError::Backend(error.to_string()))?;
                if !present {
                    return Err(PromotionError::MissingRevision(*revision));
                }
            }
        }
        Self::store_snapshot_in_transaction(&transaction, &next)
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        snapshot::store_parent_in_transaction(&transaction, next.id(), base)
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        let actual = roots::read_root_in_transaction(&transaction, &name)
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        if actual != expected {
            return Err(PromotionError::Root(scriberig_store::RootError::Conflict {
                name,
                expected,
                actual,
            }));
        }
        roots::write_root_in_transaction(&transaction, &name, next.id())
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        related_write(&transaction).map_err(PromotionError::Backend)?;
        transaction
            .commit()
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        Ok(next.id())
    }
}
