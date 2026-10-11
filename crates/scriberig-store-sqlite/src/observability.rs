//! Implements SQLite storage observability and derived-index maintenance.

use scriberig_store::{
    IndexStatus, ProtectedRoots, RootStatus, StoreObservability, StoreOperationError, StoreStats,
};

use crate::SqliteStore;

impl StoreObservability for SqliteStore {
    /// Returns authoritative row counts from SQLite.
    ///
    /// # Returns
    /// Available SQLite storage statistics.
    ///
    /// # Errors
    /// Returns a backend error when statistics cannot be read.
    fn stats(&self) -> Result<StoreStats, StoreOperationError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreOperationError::backend("lock poisoned"))?;
        let count = |table: &str| {
            connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0).and_then(|value| {
                        u64::try_from(value).map_err(|_| rusqlite::Error::InvalidQuery)
                    })
                })
                .map_err(|error| StoreOperationError::backend(error.to_string()))
        };
        Ok(StoreStats {
            revisions: Some(count("revisions")?),
            snapshots: Some(count("snapshots")?),
            named_roots: Some(count("roots")?),
            ..StoreStats::default()
        })
    }

    /// Returns counts annotated with SQLite reachability.
    ///
    /// # Parameters
    /// - `protected`: Snapshot roots used for reachability counts.
    ///
    /// # Returns
    /// Authoritative counts and marked-object counts.
    ///
    /// # Errors
    /// Returns a backend error when integrity inspection cannot complete.
    fn stats_for(&self, protected: &ProtectedRoots) -> Result<StoreStats, StoreOperationError> {
        let mut stats = self.stats()?;
        let report = self
            .integrity_check(protected)
            .map_err(|error| StoreOperationError::backend(error.to_string()))?;
        let total = stats.revisions.unwrap_or(0) + stats.snapshots.unwrap_or(0);
        let unreachable =
            (report.unreachable.revisions.len() + report.unreachable.snapshots.len()) as u64;
        stats.reachable_objects = Some(total.saturating_sub(unreachable));
        stats.unreachable_objects = Some(unreachable);
        Ok(stats)
    }

    /// Lists SQLite roots in deterministic name order.
    ///
    /// # Returns
    /// Root status records ordered by root name.
    ///
    /// # Errors
    /// Returns a backend error when roots cannot be read.
    fn root_statuses(&self) -> Result<Vec<RootStatus>, StoreOperationError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StoreOperationError::backend("lock poisoned"))?;
        let mut statement = connection
            .prepare("SELECT name, snapshot_id FROM roots ORDER BY name")
            .map_err(|error| StoreOperationError::backend(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                let name: String = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                Ok((name, bytes))
            })
            .map_err(|error| StoreOperationError::backend(error.to_string()))?;
        let mut statuses = Vec::new();
        for row in rows {
            let (name, bytes) =
                row.map_err(|error| StoreOperationError::backend(error.to_string()))?;
            if bytes.len() != 32 {
                return Err(StoreOperationError {
                    category: scriberig_store::StoreErrorCategory::InvalidEncoding,
                    message: "invalid root snapshot identifier".to_owned(),
                });
            }
            let mut id = [0; 32];
            id.copy_from_slice(&bytes);
            statuses.push(RootStatus {
                name: scriberig_core::RootName::new(name).map_err(|error| StoreOperationError {
                    category: scriberig_store::StoreErrorCategory::InvalidEncoding,
                    message: format!("{error:?}"),
                })?,
                snapshot: Some(scriberig_core::SnapshotId::new(id)),
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
    fn rebuild_indexes(&self) -> Result<IndexStatus, StoreOperationError> {
        self.rebuild_derived_indexes()
            .map_err(|error| StoreOperationError::backend(error.to_string()))?;
        Ok(IndexStatus { consistent: true })
    }
}
