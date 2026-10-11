//! Durable named-root persistence for the SQLite backend.

use crate::SqliteStore;
use rusqlite::{OptionalExtension, params};
use scriberig_core::{RootName, SnapshotId};
use scriberig_store::{RootError, RootStore};

impl RootStore for SqliteStore {
    /// # Parameters
    /// - `name`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    /// Reads a named root.
    fn read_root(&self, name: &RootName) -> Option<SnapshotId> {
        let connection = self.connection.lock().ok()?;
        let bytes: Vec<u8> = connection
            .query_row(
                "SELECT snapshot_id FROM roots WHERE name=?",
                params![name.as_str()],
                |row| row.get(0),
            )
            .optional()
            .ok()??;
        if bytes.len() != 32 {
            return None;
        }
        let mut value = [0; 32];
        value.copy_from_slice(&bytes);
        Some(SnapshotId::new(value))
    }

    /// Unconditionally points a named root at an immutable snapshot.
    fn set_root(&self, name: RootName, snapshot: SnapshotId) -> Result<(), RootError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| RootError::Backend("lock poisoned".to_owned()))?;
        write_root_in_connection(&connection, &name, snapshot)
            .map_err(|error| RootError::Backend(error.to_string()))
    }
    /// # Parameters
    /// - `name`: Operation input.
    /// - `expected`: Operation input.
    /// - `new`: Operation input.
    ///
    /// # Returns
    /// The requested storage result.
    ///
    /// # Errors
    /// Returns a backend error when SQLite rejects the operation.
    /// Atomically promotes a root when its expected value matches.
    fn compare_and_set_root(
        &self,
        name: RootName,
        expected: Option<SnapshotId>,
        new: SnapshotId,
    ) -> Result<(), RootError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| RootError::Backend("lock poisoned".to_owned()))?;
        let transaction = connection
            .transaction()
            .map_err(|error| RootError::Backend(error.to_string()))?;
        let actual = read_root_in_transaction(&transaction, &name)
            .map_err(|error| RootError::Backend(error.to_string()))?;
        if actual != expected {
            return Err(RootError::Conflict {
                name,
                expected,
                actual,
            });
        }
        write_root_in_transaction(&transaction, &name, new)
            .map_err(|error| RootError::Backend(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| RootError::Backend(error.to_string()))
    }
}

/// Reads a root inside an existing transaction.
///
/// # Parameters
/// - `transaction`: Transaction containing the root table.
/// - `name`: Root name to read.
/// - `crate`: Internal visibility marker.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// The current snapshot assigned to the root, when present.
///
/// # Errors
/// Returns the SQLite query error.
pub(crate) fn read_root_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    name: &RootName,
) -> rusqlite::Result<Option<SnapshotId>> {
    let bytes: Option<Vec<u8>> = transaction
        .query_row(
            "SELECT snapshot_id FROM roots WHERE name=?",
            params![name.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    Ok(bytes.and_then(|bytes| {
        if bytes.len() == 32 {
            let mut value = [0; 32];
            value.copy_from_slice(&bytes);
            Some(SnapshotId::new(value))
        } else {
            None
        }
    }))
}

/// Writes a root inside an existing transaction.
///
/// # Parameters
/// - `transaction`: Transaction receiving the root update.
/// - `name`: Root name to update.
/// - `snapshot`: Snapshot identity to assign.
/// - `crate`: Internal visibility marker.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Unit after the update is accepted.
///
/// # Errors
/// Returns the SQLite update error.
pub(crate) fn write_root_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    name: &RootName,
    snapshot: SnapshotId,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO roots(name,snapshot_id) VALUES (?,?)
         ON CONFLICT(name) DO UPDATE SET snapshot_id=excluded.snapshot_id",
        params![name.as_str(), snapshot.as_bytes().as_slice()],
    )?;
    Ok(())
}

/// Writes a root without comparing a previously observed value.
fn write_root_in_connection(
    connection: &rusqlite::Connection,
    name: &RootName,
    snapshot: SnapshotId,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO roots(name,snapshot_id) VALUES (?,?)
         ON CONFLICT(name) DO UPDATE SET snapshot_id=excluded.snapshot_id",
        params![name.as_str(), snapshot.as_bytes().as_slice()],
    )?;
    Ok(())
}
