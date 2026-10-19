//! Rebuilds SQLite indexes derived from canonical snapshot-map nodes.

use rusqlite::{OptionalExtension, params};
use scriberig_core::SnapshotId;

use crate::{SqliteStore, integrity, snapshot_integrity};

impl SqliteStore {
    /// Rebuilds the flat snapshot-entry index from persistent map nodes.
    ///
    /// Unit after the derived index transaction commits.
    ///
    /// Returns an error when canonical nodes cannot be inspected or the index
    /// transaction cannot commit.
    ///
    /// # Panics
    /// Panics only if a checked SQLite identifier length changes before the
    /// identifier conversion.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn rebuild_derived_indexes(&self) -> rusqlite::Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut rebuilt = Vec::new();
        let mut statement = connection.prepare("SELECT id FROM snapshots ORDER BY id")?;
        let snapshots = statement.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
        for snapshot in snapshots {
            let snapshot = snapshot?;
            if snapshot.len() != 32 {
                return Err(rusqlite::Error::InvalidQuery);
            }
            let node: Option<Vec<u8>> = connection
                .query_row(
                    "SELECT node_id FROM snapshot_node_roots WHERE snapshot_id=?",
                    rusqlite::params![&snapshot],
                    |row| row.get(0),
                )
                .optional()?;
            let entries = if let Some(node) = node {
                snapshot_integrity::inspect_nodes(&connection, &node)
                    .map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(
                            format!("invalid snapshot index source: {error:?}"),
                        )))
                    })?
                    .entries
            } else {
                let mut snapshot_bytes = [0; 32];
                snapshot_bytes.copy_from_slice(&snapshot);
                let snapshot_id = SnapshotId::new(snapshot_bytes);
                integrity::load_legacy_entries(&connection, snapshot_id)?
            };
            let mut snapshot_bytes = [0; 32];
            snapshot_bytes.copy_from_slice(&snapshot);
            rebuilt.push((SnapshotId::new(snapshot_bytes), entries));
        }
        drop(statement);
        let transaction = connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM snapshot_entries", [])?;
        for (snapshot, entries) in rebuilt {
            for (entity, revision) in entries {
                transaction.execute(
                    "INSERT INTO snapshot_entries(snapshot_id,entity,revision)
                     VALUES (?,?,?)",
                    params![
                        snapshot.as_bytes().as_slice(),
                        entity.as_bytes().as_slice(),
                        revision.as_bytes().as_slice()
                    ],
                )?;
            }
        }
        transaction.commit()
    }
}
