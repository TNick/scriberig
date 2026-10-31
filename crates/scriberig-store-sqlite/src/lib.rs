//! Provides the durable SQLite implementation of Scriberig storage contracts.

#![allow(clippy::needless_pass_by_value)]

mod connection;
mod gc;
mod indexes;
mod integrity;
mod integrity_nodes;
mod observability;
mod project_files;
mod project_rows;
mod projects;
mod promotion;
mod reachability;
mod recovery;
mod revision;
mod roots;
mod schema;
mod snapshot;
mod snapshot_integrity;
mod snapshot_node_store;
mod snapshot_relations;
mod storage;
mod workflow;
mod workflow_candidates;

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OpenFlags};
use scriberig_core::{ProjectId, RevisionId, RootName, SnapshotId};
use scriberig_store::{Revision, Snapshot, SnapshotError};

/// A durable SQLite-backed revision, snapshot, and root store.
#[derive(Debug)]
pub struct SqliteStore {
    connection: Arc<Mutex<Connection>>,
}

pub use project_files::{ProjectEntityRecord, ProjectFileDescriptor, ProjectFileRecord};
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
    /// Returns the main database path when this store is file-backed.
    ///
    /// # Parameters
    /// - `self`: SQLite store to inspect.
    ///
    /// # Returns
    /// The canonical path reported by SQLite, or `None` for an in-memory store.
    ///
    /// # Errors
    /// Returns a SQLite error when the connection cannot inspect its database list.
    pub fn database_path(&self) -> rusqlite::Result<Option<std::path::PathBuf>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let path: String = connection.query_row("PRAGMA database_list", [], |row| row.get(2))?;
        if path.is_empty() {
            Ok(None)
        } else {
            Ok(Some(std::path::PathBuf::from(path)))
        }
    }

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

    /// Opens an SQLite store using explicit connection flags.
    ///
    /// # Parameters
    /// - `path`: SQLite path or URI.
    /// - `flags`: SQLite connection flags.
    ///
    /// # Returns
    /// An initialized durable store.
    ///
    /// # Errors
    /// Returns a SQLite initialization error.
    pub fn open_with_flags(path: impl AsRef<Path>, flags: OpenFlags) -> rusqlite::Result<Self> {
        Self::from_connection(connection::configure(Connection::open_with_flags(
            path, flags,
        )?)?)
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

    /// Shares this store's connection with another repository layer.
    ///
    /// Shared ownership lets semantic and product rows participate in the
    /// same SQLite transaction.
    ///
    /// # Parameters
    /// - `self`: Universal store owning the connection.
    ///
    /// # Returns
    /// A cloneable synchronized handle to the connection.
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

    /// Stores a snapshot and its persistent map root in an open transaction.
    ///
    /// - `transaction`: Transaction receiving the immutable records.
    /// - `snapshot`: Snapshot to persist.
    /// - `crate`: Internal visibility marker.
    ///
    /// Unit after the records are prepared.
    ///
    /// Returns the SQLite insertion error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(crate) fn store_snapshot_in_transaction(
        transaction: &rusqlite::Transaction<'_>,
        snapshot: &Snapshot,
    ) -> rusqlite::Result<()> {
        snapshot::store_in_transaction(transaction, snapshot)
    }
}

/// Initializes a development database path and its schema.
///
/// - `path`: SQLite database path.
///
/// Unit after initialization.
///
/// Returns a database initialization error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn initialize_development_database(
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    SqliteStore::open(path).map(|_| ()).map_err(Into::into)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
