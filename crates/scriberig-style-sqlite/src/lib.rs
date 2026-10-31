//! Durable SQLite storage for projection style configuration.
//!
//! Style configuration is presentation and policy state, so it lives in its own
//! table and its own revision sequence: writing a preference or a workspace
//! materialization policy never creates canonical semantic state, and the
//! settings survive a process restart.

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::{Connection, OptionalExtension as _, params};
use scriberig_app::{ApplicationError, ProjectionStyleStore, StoredStyle, StyleEntryKey};
use sr_projection::StyleValue;

/// Durable projection style configuration repository.
///
/// # Public members
/// - `open`: Opens or creates the repository at a database path.
/// - `open_in_memory`: Builds an isolated repository for tests.
///
/// # Private members
/// - `connection`: Connection guarded for serialized configuration writes.
pub struct SqliteProjectionStyleStore {
    /// Connection guarded for serialized configuration operations.
    connection: Mutex<Connection>,
}

/// Counter naming isolated in-memory databases for this process.
static NEXT_MEMORY_DATABASE: AtomicU64 = AtomicU64::new(1);

impl SqliteProjectionStyleStore {
    /// Opens or creates a style configuration repository at `path`.
    ///
    /// - `path`: SQLite database path, which may be shared with other stores.
    ///
    /// # Returns
    /// The initialized repository.
    ///
    /// # Errors
    /// Returns a SQLite error when the database cannot be opened or the settings
    /// table cannot be created.
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    /// Creates an isolated in-memory style configuration repository.
    ///
    /// # Returns
    /// The initialized repository.
    ///
    /// # Errors
    /// Returns a SQLite error when the repository cannot be initialized.
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let id = NEXT_MEMORY_DATABASE.fetch_add(1, Ordering::Relaxed);
        let uri = format!("file:scriberig-style-{id}?mode=memory&cache=shared");
        let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
            | rusqlite::OpenFlags::SQLITE_OPEN_URI
            | rusqlite::OpenFlags::SQLITE_OPEN_SHARED_CACHE;
        Self::from_connection(Connection::open_with_flags(uri, flags)?)
    }

    /// Initializes the settings table on one connection.
    ///
    /// - `connection`: Connection to initialize.
    ///
    /// # Returns
    /// The initialized repository.
    ///
    /// # Errors
    /// Returns a SQLite error when the settings table cannot be created.
    fn from_connection(connection: Connection) -> rusqlite::Result<Self> {
        connection.execute_batch("PRAGMA busy_timeout = 5000;")?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS projection_style_entries (
                 scope TEXT NOT NULL,
                 scope_id TEXT NOT NULL,
                 projection_kind TEXT NOT NULL,
                 value BLOB NOT NULL,
                 revision INTEGER NOT NULL
             );
             CREATE UNIQUE INDEX IF NOT EXISTS projection_style_entry_identity
                 ON projection_style_entries (scope, scope_id, projection_kind);",
        )?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Locks the connection, reporting a poisoned lock as an internal error.
    ///
    /// - `self`: Repository to lock.
    ///
    /// # Returns
    /// The guarded connection.
    ///
    /// # Errors
    /// Returns an internal application error when the lock is poisoned.
    fn guarded(&self) -> Result<std::sync::MutexGuard<'_, Connection>, ApplicationError> {
        self.connection
            .lock()
            .map_err(|error| ApplicationError::Internal {
                message: format!("projection style store lock poisoned: {error}"),
            })
    }
}

impl ProjectionStyleStore for SqliteProjectionStyleStore {
    /// Reads one stored style setting.
    ///
    /// - `key`: Configuration key to read.
    ///
    /// # Returns
    /// The stored setting when one exists.
    ///
    /// # Errors
    /// Returns an internal application error when the read fails.
    fn read(&self, key: &StyleEntryKey) -> Result<Option<StoredStyle>, ApplicationError> {
        let guard = self.guarded()?;
        let stored = guard
            .query_row(
                "SELECT value, revision FROM projection_style_entries
                 WHERE scope = ?1 AND scope_id = ?2 AND projection_kind = ?3",
                params![
                    key.scope.to_string(),
                    key.scope_id.as_str(),
                    key.projection_kind.as_str()
                ],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()
            .map_err(|error| storage_error(key, &error))?;
        Ok(stored.map(|(value, revision)| StoredStyle {
            value: StyleValue::new(value),
            revision: u64::try_from(revision).unwrap_or(u64::MAX),
        }))
    }

    /// Writes one setting only when the caller's revision is still current.
    ///
    /// - `key`: Configuration key to write.
    /// - `expected_revision`: Revision observed by the caller, or `None` to write
    ///   without a precondition.
    /// - `value`: New style input.
    ///
    /// # Returns
    /// The authoritative stored setting after the write.
    ///
    /// # Errors
    /// Returns `ApplicationError::SemanticConflict` when the revision is stale and
    /// an internal application error when the write fails.
    fn compare_and_swap(
        &self,
        key: &StyleEntryKey,
        expected_revision: Option<u64>,
        value: StyleValue,
    ) -> Result<StoredStyle, ApplicationError> {
        let mut guard = self.guarded()?;
        let transaction = guard
            .transaction()
            .map_err(|error| storage_error(key, &error))?;
        let current = current_revision(&transaction, key)?;
        if let Some(expected) = expected_revision
            && current != expected
        {
            return Err(conflict(key));
        }
        let revision = current + 1;
        transaction
            .execute(
                "INSERT INTO projection_style_entries
                     (scope, scope_id, projection_kind, value, revision)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (scope, scope_id, projection_kind)
                 DO UPDATE SET value = excluded.value, revision = excluded.revision",
                params![
                    key.scope.to_string(),
                    key.scope_id.as_str(),
                    key.projection_kind.as_str(),
                    value.as_bytes(),
                    i64::try_from(revision).unwrap_or(i64::MAX)
                ],
            )
            .map_err(|error| storage_error(key, &error))?;
        transaction
            .commit()
            .map_err(|error| storage_error(key, &error))?;
        Ok(StoredStyle { value, revision })
    }

    /// Removes one setting when it still matches the caller's revision.
    ///
    /// - `key`: Configuration key to remove.
    /// - `expected_revision`: Revision observed by the caller, or `None` to clear
    ///   without a precondition.
    ///
    /// # Returns
    /// Unit when the setting is absent or was removed.
    ///
    /// # Errors
    /// Returns `ApplicationError::SemanticConflict` when the revision is stale and
    /// an internal application error when the removal fails.
    fn clear(
        &self,
        key: &StyleEntryKey,
        expected_revision: Option<u64>,
    ) -> Result<(), ApplicationError> {
        let mut guard = self.guarded()?;
        let transaction = guard
            .transaction()
            .map_err(|error| storage_error(key, &error))?;
        let current = current_revision(&transaction, key)?;
        if let Some(expected) = expected_revision
            && current != expected
        {
            return Err(conflict(key));
        }
        transaction
            .execute(
                "DELETE FROM projection_style_entries
                 WHERE scope = ?1 AND scope_id = ?2 AND projection_kind = ?3",
                params![
                    key.scope.to_string(),
                    key.scope_id.as_str(),
                    key.projection_kind.as_str()
                ],
            )
            .map_err(|error| storage_error(key, &error))?;
        transaction
            .commit()
            .map_err(|error| storage_error(key, &error))?;
        Ok(())
    }
}

/// Reads the current revision of one setting inside an open transaction.
///
/// - `transaction`: Open transaction holding the read.
/// - `key`: Configuration key to read.
///
/// # Returns
/// The stored revision, or zero when the setting is absent.
///
/// # Errors
/// Returns an internal application error when the read fails.
fn current_revision(
    transaction: &rusqlite::Transaction<'_>,
    key: &StyleEntryKey,
) -> Result<u64, ApplicationError> {
    let current = transaction
        .query_row(
            "SELECT revision FROM projection_style_entries
             WHERE scope = ?1 AND scope_id = ?2 AND projection_kind = ?3",
            params![
                key.scope.to_string(),
                key.scope_id.as_str(),
                key.projection_kind.as_str()
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| storage_error(key, &error))?;
    Ok(current.map_or(0, |revision| u64::try_from(revision).unwrap_or(0)))
}

/// Builds the conflict reported for a stale revision precondition.
///
/// - `key`: Configuration key whose precondition failed.
///
/// # Returns
/// The semantic conflict naming the setting.
fn conflict(key: &StyleEntryKey) -> ApplicationError {
    ApplicationError::SemanticConflict {
        subject: format!(
            "projection style {}:{}:{}",
            key.scope,
            key.scope_id,
            key.projection_kind.as_str()
        ),
    }
}

/// Converts a storage failure into an internal application error.
///
/// - `key`: Configuration key being operated on.
/// - `error`: SQLite failure to describe.
///
/// # Returns
/// The internal application error carrying the storage diagnostic.
fn storage_error(key: &StyleEntryKey, error: &rusqlite::Error) -> ApplicationError {
    ApplicationError::Internal {
        message: format!(
            "projection style store failed for {}:{}:{}: {error}",
            key.scope,
            key.scope_id,
            key.projection_kind.as_str()
        ),
    }
}
