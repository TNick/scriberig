//! Provides the durable SQLite implementation of Scriberig storage contracts.

#![allow(clippy::needless_pass_by_value)]

mod connection;
mod development;
mod gc;
mod indexes;
mod integrity;
mod integrity_nodes;
mod observability;
mod project_files;
mod project_rows;
mod projects;
mod reachability;
mod recovery;
mod revision;
mod roots;
mod schema;
mod snapshot;
mod snapshot_integrity;
mod storage;
mod workflow;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, TransactionBehavior};
use scriberig_core::{ProjectId, RevisionId, RootName, SnapshotId};
use scriberig_store::{Revision, SnapshotError};

/// A durable SQLite-backed revision, snapshot, and root store.
#[derive(Debug)]
pub struct SqliteStore {
    connection: Arc<Mutex<Connection>>,
}

pub use development::initialize_development_database;
pub use project_files::{ProjectEntityRecord, ProjectFileRecord};
pub use projects::{ProjectRecord, ProjectStatus};
pub use workflow::{
    ProjectCandidateRecord, ProjectEffectPathRecord, ProjectEffectRecord, ProjectSourceEffectRecord,
};

/// Reports a failure while atomically preparing and promoting a snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PromotionError {
    /// The base snapshot or change application was invalid.
    Snapshot(SnapshotError),
    /// The named root was stale or could not be updated.
    Root(scriberig_store::RootError),
    /// SQLite rejected the transaction.
    Backend(String),
    /// A snapshot refers to a revision absent from the durable store.
    MissingRevision(RevisionId),
}

impl SqliteStore {
    /// - `path`: Operation input.
    /// - `rusqlite`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// Opens or creates a database at a filesystem path.
    ///
    /// - `path`: SQLite database path.
    ///
    /// An initialized durable store.
    ///
    /// Returns a SQLite initialization error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        Self::from_connection(connection::configure(Connection::open(path)?)?)
    }
    /// - `rusqlite`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// Opens an isolated in-memory database.
    ///
    /// An initialized test store.
    ///
    /// Returns a SQLite initialization error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::from_connection(connection::configure(Connection::open_in_memory()?)?)
    }
    /// Internal SQLite storage operation.
    /// - `crate`: Internal visibility marker.
    /// - `connection`: Operation input.
    /// - `rusqlite`: Operation input.
    ///
    /// The requested storage result.
    ///
    /// Returns a backend error when SQLite rejects the operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn from_connection(connection: Connection) -> rusqlite::Result<Self> {
        let mut connection = connection;
        schema::initialize(&mut connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Shares this store's SQLite connection with a companion repository.
    ///
    /// The connection is useful when two SQLite-backed layers must commit
    /// their rows in the same transaction.
    ///
    /// # Parameters
    /// - `self`: Universal SQLite store owning the connection.
    ///
    /// A cloneable synchronized connection handle.
    /// # Returns
    /// The shared connection handle.
    #[must_use]
    pub fn shared_connection(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.connection)
    }

    /// Returns the detected backend storage schema version.
    ///
    /// The schema version recorded in the database.
    ///
    /// Returns a SQLite query error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn schema_version(&self) -> rusqlite::Result<u32> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        connection.query_row(
            "SELECT value FROM metadata WHERE key='schema_version'",
            [],
            |row| {
                let value: String = row.get(0)?;
                value
                    .parse::<u32>()
                    .map_err(|_| rusqlite::Error::InvalidQuery)
            },
        )
    }

    /// Applies changes and promotes a named root in one SQLite transaction.
    ///
    /// - `name`: Named root to promote.
    /// - `expected`: Root value required for promotion.
    /// - `base`: Existing snapshot to change.
    /// - `changes`: Mapping changes for the successor snapshot.
    ///
    /// The promoted immutable snapshot identity.
    ///
    /// Returns a snapshot, stale-root, or transaction error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn apply_changes_and_promote(
        &self,
        name: scriberig_core::RootName,
        expected: Option<SnapshotId>,
        base: SnapshotId,
        changes: &[scriberig_store::SnapshotChange],
    ) -> Result<SnapshotId, PromotionError> {
        self.apply_revisions_and_promote(name, expected, base, &[], changes)
    }

    /// Inserts revisions, creates a snapshot, and promotes its named root atomically.
    ///
    /// - `name`: Named root to promote.
    /// - `expected`: Root value required for promotion.
    /// - `base`: Existing snapshot to change.
    /// - `revisions`: New immutable revisions to insert idempotently.
    /// - `changes`: Mapping changes for the successor snapshot.
    ///
    /// The promoted immutable snapshot identity.
    ///
    /// Returns a snapshot, missing-revision, stale-root, or transaction error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn apply_revisions_and_promote(
        &self,
        name: scriberig_core::RootName,
        expected: Option<SnapshotId>,
        base: SnapshotId,
        revisions: &[Revision],
        changes: &[scriberig_store::SnapshotChange],
    ) -> Result<SnapshotId, PromotionError> {
        self.apply_revisions_and_promote_with(name, expected, base, revisions, changes, |_| Ok(()))
    }

    /// Promotes a snapshot and commits related SQLite rows in the same transaction.
    ///
    /// # Parameters
    /// - `name`: Named root to promote.
    /// - `expected`: Root value required for promotion.
    /// - `base`: Existing snapshot to change.
    /// - `revisions`: New immutable revisions to insert idempotently.
    /// - `changes`: Mapping changes to apply to the base snapshot.
    /// - `commit_related`: Related writes executed after root validation and
    ///   before the transaction commits.
    ///
    /// The promoted immutable snapshot identity.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns a snapshot, missing-revision, stale-root, or transaction error.
    pub fn apply_revisions_and_promote_with<F>(
        &self,
        name: scriberig_core::RootName,
        expected: Option<SnapshotId>,
        base: SnapshotId,
        revisions: &[Revision],
        changes: &[scriberig_store::SnapshotChange],
        commit_related: F,
    ) -> Result<SnapshotId, PromotionError>
    where
        F: FnOnce(&rusqlite::Transaction<'_>) -> Result<(), String>,
    {
        let source = self.snapshot(base).map_err(PromotionError::Snapshot)?;
        let next = source.apply(changes.iter().copied());
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| PromotionError::Backend("lock poisoned".to_owned()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        for revision in revisions {
            revision::insert_in_transaction(&transaction, revision)
                .map_err(|error| PromotionError::Backend(error.to_string()))?;
        }
        for change in changes {
            if let scriberig_store::SnapshotChange::Set(_, revision) = change {
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
        if let Err(error) = Self::store_snapshot_in_transaction(&transaction, &next) {
            return Err(PromotionError::Backend(error.to_string()));
        }
        if let Err(error) = snapshot::store_parent_in_transaction(&transaction, next.id(), base) {
            return Err(PromotionError::Backend(error.to_string()));
        }
        let actual = roots::read_root_in_transaction(&transaction, &name)
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        if actual != expected {
            drop(transaction);
            return Err(PromotionError::Root(scriberig_store::RootError::Conflict {
                name,
                expected,
                actual,
            }));
        }
        roots::write_root_in_transaction(&transaction, &name, next.id())
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        commit_related(&transaction).map_err(PromotionError::Backend)?;
        transaction
            .commit()
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        Ok(next.id())
    }

    /// Promotes a project snapshot and its catalog metadata atomically.
    ///
    /// - `project`: Project whose snapshot and ownership are being replaced.
    /// - `name`: Named root receiving the promoted snapshot.
    /// - `expected`: Root value required for compare-and-swap promotion.
    /// - `base`: Existing snapshot used as the change base.
    /// - `revisions`: Immutable revisions to insert before promotion.
    /// - `changes`: Snapshot map changes to apply.
    /// - `entities`: Ownership rows matching the resulting project snapshot.
    ///
    /// The promoted immutable snapshot identity.
    ///
    /// Returns a snapshot, stale-root, missing-revision, or transaction error.
    #[allow(clippy::too_many_arguments)]
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn apply_project_import(
        &self,
        project: ProjectId,
        name: RootName,
        expected: Option<SnapshotId>,
        base: SnapshotId,
        revisions: &[Revision],
        changes: &[scriberig_store::SnapshotChange],
        entities: &[ProjectEntityRecord],
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
            if let scriberig_store::SnapshotChange::Set(_, revision) = change {
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
        transaction
            .execute(
                "DELETE FROM project_entities WHERE project_id=?",
                rusqlite::params![project.as_bytes().as_slice()],
            )
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        for entity in entities {
            transaction
                .execute(
                    "INSERT INTO project_entities(project_id,relative_path,entity,kind,name)
                     VALUES (?,?,?,?,?)",
                    rusqlite::params![
                        entity.project_id.as_bytes().as_slice(),
                        entity.relative_path,
                        entity.entity.as_bytes().as_slice(),
                        entity.kind,
                        entity.name,
                    ],
                )
                .map_err(|error| PromotionError::Backend(error.to_string()))?;
        }
        transaction
            .execute(
                "UPDATE projects SET status='imported',current_snapshot=?,updated_at=datetime('now')
                 WHERE id=?",
                rusqlite::params![
                    next.id().as_bytes().as_slice(),
                    project.as_bytes().as_slice(),
                ],
            )
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        Ok(next.id())
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
