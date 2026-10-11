//! Rebuilds SQLite indexes derived from canonical snapshot-map nodes.

use rusqlite::params;
use scriberig_core::SnapshotId;

use crate::{SqliteStore, snapshot_integrity};

impl SqliteStore {
    /// Rebuilds the flat snapshot-entry index from persistent map nodes.
    ///
    /// # Returns
    /// Unit after the derived index transaction commits.
    ///
    /// # Errors
    /// Returns an error when canonical nodes cannot be inspected or the index
    /// transaction cannot commit.
    pub fn rebuild_derived_indexes(&self) -> rusqlite::Result<()> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut rebuilt = Vec::new();
        let mut statement = connection
            .prepare("SELECT snapshot_id, node_id FROM snapshot_node_roots ORDER BY snapshot_id")?;
        let roots = statement.query_map([], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?))
        })?;
        for root in roots {
            let (snapshot, node) = root?;
            if snapshot.len() != 32 {
                return Err(rusqlite::Error::InvalidQuery);
            }
            let walk = snapshot_integrity::inspect_nodes(&connection, &node).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(format!(
                    "invalid snapshot index source: {error:?}"
                ))))
            })?;
            let mut snapshot_bytes = [0; 32];
            snapshot_bytes.copy_from_slice(&snapshot);
            rebuilt.push((SnapshotId::new(snapshot_bytes), walk.entries));
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
