//! Resolves exact plan revisions from universal snapshot history.

use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::Plan;
use scriberig_store::{RootStore, SnapshotStore};

use crate::{SqliteSessionStore, semantic_plans::plan_root};

impl SqliteSessionStore {
    /// Resolves a plan projection at one exact root-history snapshot.
    ///
    /// # Parameters
    /// - `session`: Owning durable session identity.
    /// - `plan_id`: Stable plan identity.
    /// - `snapshot`: Exact universal snapshot to resolve.
    ///
    /// # Returns
    /// The historical projection, or `None` when outside plan history.
    ///
    /// # Errors
    /// Returns a backend diagnostic when semantic history cannot be read.
    pub(crate) fn resolve_plan_snapshot(
        &self,
        session: EntityId,
        plan_id: EntityId,
        snapshot: SnapshotId,
    ) -> Result<Option<Plan>, String> {
        let root = plan_root(session, plan_id)?;
        let Some(current_snapshot) = self.semantic_store.read_root(&root) else {
            return Ok(None);
        };
        let ancestors = self
            .semantic_store
            .ancestors(current_snapshot)
            .map_err(|error| format!("plan history read failed: {error:?}"))?;
        if snapshot != current_snapshot && !ancestors.contains(&snapshot) {
            return Ok(None);
        }
        SqliteSessionStore::load_plan_at_snapshot(self, session, plan_id, snapshot).map(Some)
    }
}
