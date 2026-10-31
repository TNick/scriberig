//! Persists rebuildable effective plan-node rollups in SQLite.

use std::collections::BTreeMap;

use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::PlanNodeRollup;
use scriberig_store::RootStore;

use super::plan_root;
use crate::SqliteSessionStore;

#[path = "rollup_index/read.rs"]
mod read;
#[path = "rollup_index/write.rs"]
mod write;
use read::read_rollup_rows;
pub(super) use write::{replace_in_transaction, write_rollup_metadata, write_rollup_row};

impl SqliteSessionStore {
    /// Rebuilds current node and rollup indexes from canonical plan state.
    ///
    /// # Parameters
    /// - `self`: SQLite session repository.
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    ///
    /// # Returns
    /// The exact snapshot indexed after the rebuild.
    ///
    /// # Errors
    /// Returns a diagnostic when canonical state or the index transaction
    /// cannot be read or written.
    pub fn rebuild_plan_rollup_index(
        &self,
        session: EntityId,
        plan: EntityId,
    ) -> Result<SnapshotId, String> {
        let root = plan_root(session, plan)?;
        let snapshot = self
            .semantic_store
            .read_root(&root)
            .ok_or_else(|| "plan semantic root is absent".to_owned())?;
        self.rebuild_plan_node_index(session, plan)?;
        Ok(snapshot)
    }

    /// Reads current normalized rollups only when they match `snapshot`.
    ///
    /// # Parameters
    /// - `self`: SQLite session repository.
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    /// - `snapshot`: Exact plan snapshot expected in the index.
    ///
    /// # Returns
    /// Snapshot-valid rollups, or `None` when the cache is absent or stale.
    ///
    /// # Errors
    /// Returns a diagnostic when normalized rows are malformed or unreadable.
    pub fn read_plan_rollup_index(
        &self,
        session: EntityId,
        plan: EntityId,
        snapshot: SnapshotId,
    ) -> Result<Option<BTreeMap<EntityId, PlanNodeRollup>>, String> {
        self.load_plan_rollups_at_snapshot(session, plan, snapshot)
    }

    /// Reads cached rollups only when all rows match the requested snapshot.
    ///
    /// # Parameters
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    /// - `snapshot`: Exact plan snapshot requested by the caller.
    ///
    /// # Returns
    /// Cached node rollups, or `None` when the cache is absent or stale.
    ///
    /// # Errors
    /// Returns a diagnostic when the root or normalized rows cannot be read.
    pub fn load_plan_rollups_at_snapshot(
        &self,
        session: EntityId,
        plan: EntityId,
        snapshot: SnapshotId,
    ) -> Result<Option<BTreeMap<EntityId, PlanNodeRollup>>, String> {
        let root = plan_root(session, plan)?;
        if self.semantic_store.read_root(&root) != Some(snapshot) {
            return Ok(None);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        read_rollup_rows(&connection, session, plan, snapshot)
    }
}
