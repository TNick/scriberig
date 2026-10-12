//! Provides the durable SQLite implementation of Scriberig storage contracts.

#![allow(clippy::needless_pass_by_value)]

mod connection;
mod gc;
mod indexes;
mod integrity;
mod integrity_nodes;
mod observability;
mod projects;
mod reachability;
mod recovery;
mod revision;
mod roots;
mod schema;
mod snapshot;
mod snapshot_integrity;
mod storage;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;
use scriberig_core::{RevisionId, SnapshotId};
use scriberig_store::{Revision, Snapshot, SnapshotError};

/// A durable SQLite-backed revision, snapshot, and root store.
#[derive(Debug)]
pub struct SqliteStore {
    connection: Mutex<Connection>,
}

pub use projects::{ProjectRecord, ProjectStatus};

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
    /// # Parameters
    /// - `path`: Operation input.
    /// - `rusqlite`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Opens or creates a database at a filesystem path.
    ///
    /// # Parameters
    /// - `path`: SQLite database path.
    ///
    /// # Returns
    /// An initialized durable store.
    ///
    /// # Errors
    /// Returns a SQLite initialization error.
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        Self::from_connection(connection::configure(Connection::open(path)?)?)
    }
    /// # Parameters
    /// - `rusqlite`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Opens an isolated in-memory database.
    ///
    /// # Returns
    /// An initialized test store.
    ///
    /// # Errors
    /// Returns a SQLite initialization error.
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Self::from_connection(connection::configure(Connection::open_in_memory()?)?)
    }
    /// Internal SQLite storage operation.
    /// # Parameters
    /// - `crate`: Internal visibility marker.
    /// - `connection`: Operation input.
    /// - `rusqlite`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    fn from_connection(connection: Connection) -> rusqlite::Result<Self> {
        let mut connection = connection;
        schema::initialize(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Returns the detected backend storage schema version.
    ///
    /// # Returns
    /// The schema version recorded in the database.
    ///
    /// # Errors
    /// Returns a SQLite query error.
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
    /// # Parameters
    /// - `name`: Named root to promote.
    /// - `expected`: Root value required for promotion.
    /// - `base`: Existing snapshot to change.
    /// - `changes`: Mapping changes for the successor snapshot.
    ///
    /// # Returns
    /// The promoted immutable snapshot identity.
    ///
    /// # Errors
    /// Returns a snapshot, stale-root, or transaction error.
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
    /// # Parameters
    /// - `name`: Named root to promote.
    /// - `expected`: Root value required for promotion.
    /// - `base`: Existing snapshot to change.
    /// - `revisions`: New immutable revisions to insert idempotently.
    /// - `changes`: Mapping changes for the successor snapshot.
    ///
    /// # Returns
    /// The promoted immutable snapshot identity.
    ///
    /// # Errors
    /// Returns a snapshot, missing-revision, stale-root, or transaction error.
    pub fn apply_revisions_and_promote(
        &self,
        name: scriberig_core::RootName,
        expected: Option<SnapshotId>,
        base: SnapshotId,
        revisions: &[Revision],
        changes: &[scriberig_store::SnapshotChange],
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
        transaction
            .commit()
            .map_err(|error| PromotionError::Backend(error.to_string()))?;
        Ok(next.id())
    }

    /// Stores a snapshot and its persistent map root in an open transaction.
    ///
    /// # Parameters
    /// - `transaction`: Transaction receiving the immutable records.
    /// - `snapshot`: Snapshot to persist.
    /// - `crate`: Internal visibility marker.
    ///
    /// # Returns
    /// Unit after the records are prepared.
    ///
    /// # Errors
    /// Returns the SQLite insertion error.
    pub(crate) fn store_snapshot_in_transaction(
        transaction: &rusqlite::Transaction<'_>,
        snapshot: &Snapshot,
    ) -> rusqlite::Result<()> {
        snapshot::store_in_transaction(transaction, snapshot)
    }
}

/// Initializes a development database path and its schema.
///
/// # Parameters
/// - `path`: SQLite database path.
///
/// # Returns
/// Unit after initialization.
///
/// # Errors
/// Returns a database initialization error.
pub fn initialize_development_database(
    path: impl AsRef<Path>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    SqliteStore::open(path).map(|_| ()).map_err(Into::into)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
