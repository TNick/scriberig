//! Provides the durable SQLite implementation of Scriberig storage contracts.

#![allow(clippy::needless_pass_by_value)]

mod gc;
mod revision;
mod roots;
mod snapshot;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, params};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_store::{
    IndexStatus, RootStatus, Snapshot, SnapshotError, StoreError, StoreObservability, StoreStats,
};

/// A durable SQLite-backed revision, snapshot, and root store.
pub struct SqliteStore {
    connection: Mutex<Connection>,
}

impl StoreObservability for SqliteStore {
    /// Returns authoritative row counts from SQLite.
    ///
    /// # Returns
    /// Available SQLite storage statistics.
    ///
    /// # Errors
    /// Returns a backend error when statistics cannot be read.
    fn stats(&self) -> Result<StoreStats, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "lock poisoned".to_owned())?;
        let count = |table: &str| {
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0).and_then(|value| {
                        u64::try_from(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                })
                .map_err(|error| error.to_string())
        };
        Ok(StoreStats {
            revisions: Some(count("revisions")?),
            snapshots: Some(count("snapshots")?),
            named_roots: Some(count("roots")?),
            ..StoreStats::default()
        })
    }

    /// Lists SQLite roots in deterministic name order.
    ///
    /// # Returns
    /// Root status records ordered by root name.
    ///
    /// # Errors
    /// Returns a backend error when roots cannot be read.
    fn root_statuses(&self) -> Result<Vec<RootStatus>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "lock poisoned".to_owned())?;
        let mut statement = connection
            .prepare("SELECT name, snapshot_id FROM roots ORDER BY name")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                Ok((name, bytes))
            })
            .map_err(|error| error.to_string())?;
        let mut statuses = Vec::new();
        for row in rows {
            let (name, bytes) = row.map_err(|error| error.to_string())?;
            if bytes.len() != 32 {
                return Err("invalid root snapshot identifier".to_owned());
            }
            let mut id = [0; 32];
            id.copy_from_slice(&bytes);
            statuses.push(RootStatus {
                name: scriberig_core::RootName::new(name).map_err(|error| format!("{error:?}"))?,
                snapshot: Some(SnapshotId::new(id)),
            });
        }
        Ok(statuses)
    }

    /// Rebuilds SQLite derived indexes.
    ///
    /// # Returns
    /// A consistent index status after rebuilding.
    ///
    /// # Errors
    /// Returns a backend error when rebuilding fails.
    fn rebuild_indexes(&self) -> Result<IndexStatus, String> {
        self.rebuild_derived_indexes()
            .map_err(|error| error.to_string())?;
        Ok(IndexStatus { consistent: true })
    }
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
        Self::from_connection(Connection::open(path)?)
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
        Self::from_connection(Connection::open_in_memory()?)
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
        connection.execute_batch("PRAGMA foreign_keys = ON; CREATE TABLE IF NOT EXISTS metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS revisions (id BLOB PRIMARY KEY, version INTEGER NOT NULL, object_type TEXT NOT NULL, payload BLOB NOT NULL); CREATE TABLE IF NOT EXISTS snapshots (id BLOB PRIMARY KEY); CREATE TABLE IF NOT EXISTS snapshot_entries (snapshot_id BLOB NOT NULL, entity BLOB NOT NULL, revision BLOB NOT NULL, PRIMARY KEY(snapshot_id, entity)); CREATE TABLE IF NOT EXISTS roots (name TEXT PRIMARY KEY, snapshot_id BLOB NOT NULL); INSERT OR IGNORE INTO metadata(key,value) VALUES ('schema_version','1');")?;
        let version: String = connection.query_row(
            "SELECT value FROM metadata WHERE key='schema_version'",
            [],
            |row| row.get(0),
        )?;
        if version != "1" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    /// Internal SQLite storage operation.
    /// # Parameters
    /// - `crate`: Internal visibility marker.
    /// - `std`: Operation input.
    /// - `sync`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    pub(crate) fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, StoreError> {
        self.connection.lock().map_err(|_| StoreError::LockPoisoned)
    }
    /// Internal SQLite storage operation.
    /// # Parameters
    /// - `crate`: Internal visibility marker.
    /// - `id`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    pub(crate) fn snapshot(&self, id: SnapshotId) -> Result<Snapshot, SnapshotError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SnapshotError::Backend("lock poisoned".to_owned()))?;
        let mut statement = connection
            .prepare(
                "SELECT entity, revision FROM snapshot_entries WHERE snapshot_id=? ORDER BY entity",
            )
            .map_err(sql_snapshot)?;
        let rows = statement
            .query_map(params![id.as_bytes().as_slice()], |row| {
                let entity: Vec<u8> = row.get(0)?;
                let revision: Vec<u8> = row.get(1)?;
                Ok((entity, revision))
            })
            .map_err(sql_snapshot)?;
        let mut entries = BTreeMap::new();
        for row in rows {
            let (entity, revision) = row.map_err(sql_snapshot)?;
            if entity.len() != 16 || revision.len() != 32 {
                return Err(SnapshotError::Backend(
                    "invalid stored identifier length".to_owned(),
                ));
            }
            let mut e = [0; 16];
            e.copy_from_slice(&entity);
            let mut r = [0; 32];
            r.copy_from_slice(&revision);
            entries.insert(EntityId::new(e), RevisionId::new(r));
        }
        let snapshot = Snapshot::from_entries(entries);
        if snapshot.id() != id {
            return Err(SnapshotError::Backend(
                "snapshot identity mismatch".to_owned(),
            ));
        }
        Ok(snapshot)
    }
    /// Internal SQLite storage operation.
    /// # Parameters
    /// - `crate`: Internal visibility marker.
    /// - `snapshot`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    pub(crate) fn store_snapshot(&self, snapshot: &Snapshot) -> Result<(), SnapshotError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| SnapshotError::Backend("lock poisoned".to_owned()))?;
        connection
            .execute(
                "INSERT OR IGNORE INTO snapshots(id) VALUES (?)",
                params![snapshot.id().as_bytes().as_slice()],
            )
            .map_err(sql_snapshot)?;
        for (entity, revision) in snapshot.iter() {
            connection.execute("INSERT OR IGNORE INTO snapshot_entries(snapshot_id,entity,revision) VALUES (?,?,?)", params![snapshot.id().as_bytes().as_slice(), entity.as_bytes().as_slice(), revision.as_bytes().as_slice()]).map_err(sql_snapshot)?;
        }
        Ok(())
    }
}

/// Converts a SQLite error to a snapshot backend error.
///
/// # Parameters
/// - `error`: SQLite error.
///
/// # Returns
/// A typed snapshot backend error.
fn sql_snapshot(error: rusqlite::Error) -> SnapshotError {
    SnapshotError::Backend(error.to_string())
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
