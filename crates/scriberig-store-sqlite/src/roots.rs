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
        let current: Option<Vec<u8>> = transaction
            .query_row(
                "SELECT snapshot_id FROM roots WHERE name=?",
                params![name.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| RootError::Backend(error.to_string()))?;
        let actual = current.and_then(|bytes| {
            if bytes.len() == 32 {
                let mut value = [0; 32];
                value.copy_from_slice(&bytes);
                Some(SnapshotId::new(value))
            } else {
                None
            }
        });
        if actual != expected {
            return Err(RootError::Conflict {
                name,
                expected,
                actual,
            });
        }
        transaction.execute("INSERT INTO roots(name,snapshot_id) VALUES (?,?) ON CONFLICT(name) DO UPDATE SET snapshot_id=excluded.snapshot_id", params![name.as_str(), new.as_bytes().as_slice()]).map_err(|error| RootError::Backend(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| RootError::Backend(error.to_string()))
    }
}
