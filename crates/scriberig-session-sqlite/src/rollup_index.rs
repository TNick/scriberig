//! Persists rebuildable plan rollups outside canonical semantic revisions.

use std::collections::BTreeMap;

use rusqlite::{TransactionBehavior, params};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_session::{PlanNodeRollup, PlanNodeRollupIndex, SessionStore};
use scriberig_store::SnapshotStore;

use crate::SqliteSessionStore;

impl SqliteSessionStore {
    /// Rebuilds and persists rollups for the current semantic plan snapshot.
    ///
    /// - `self`: SQLite session repository owning the derived index.
    /// - `session_id`: Owning `ScribeRig` session identity.
    /// - `plan_id`: Stable semantic plan identity.
    ///
    /// The exact source snapshot recorded in every derived row.
    ///
    /// Returns a persistence diagnostic when the canonical graph is missing,
    /// invalid, or cannot be indexed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when loading, rebuilding, or storing the index fails.
    pub fn rebuild_plan_rollup_index(
        &self,
        session_id: EntityId,
        plan_id: EntityId,
    ) -> Result<SnapshotId, String> {
        let session = SessionStore::load(self, session_id)?
            .ok_or_else(|| "session is absent while rebuilding plan rollups".to_owned())?;
        let plan = session
            .plans
            .iter()
            .find(|plan| plan.id == plan_id)
            .ok_or_else(|| "plan is absent while rebuilding rollups".to_owned())?;
        let snapshot = plan
            .source_snapshot
            .ok_or_else(|| "plan has no exact semantic snapshot".to_owned())?;
        let index = PlanNodeRollupIndex::rebuild(plan, snapshot)
            .map_err(|error| format!("plan rollup rebuild failed: {error:?}"))?;
        let revisions = self
            .semantic_store
            .iter(snapshot)
            .map_err(|error| format!("plan snapshot revision read failed: {error:?}"))?
            .into_iter()
            .collect::<BTreeMap<EntityId, RevisionId>>();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "DELETE FROM plan_node_rollup_index WHERE session_id=?1 AND plan_id=?2",
                params![session_id.to_string(), plan_id.to_string()],
            )
            .map_err(|error| error.to_string())?;
        for rollup in index.all().values() {
            let node = plan
                .nodes
                .iter()
                .find(|node| node.id == rollup.node)
                .ok_or_else(|| "rollup points to a missing plan node".to_owned())?;
            let parent = match node.parent {
                Some(scriberig_session::PlanNodeParent::Node(parent)) => Some(parent.to_string()),
                Some(scriberig_session::PlanNodeParent::Branch(_)) | None => None,
            };
            let node_revision = revisions
                .get(&node.id)
                .ok_or_else(|| "plan node is absent from its source snapshot".to_owned())?;
            let payload = serde_json::to_vec(rollup)
                .map_err(|error| format!("plan rollup encoding failed: {error}"))?;
            transaction
                .execute(
                    "INSERT INTO plan_node_rollup_index (
                         session_id, plan_id, node_id, parent_node_id,
                         node_revision, direct_status, effective_status, source_snapshot,
                         evaluator_version, payload
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        session_id.to_string(),
                        plan_id.to_string(),
                        rollup.node.to_string(),
                        parent,
                        node_revision.to_string(),
                        format!("{:?}", rollup.direct_status),
                        format!("{:?}", rollup.effective_status),
                        index.source_snapshot().to_string(),
                        i64::from(index.evaluator_version()),
                        payload,
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(snapshot)
    }

    /// Reads derived rows only when they match the requested exact snapshot.
    ///
    /// - `self`: SQLite session repository owning the derived index.
    /// - `session_id`: Owning `ScribeRig` session identity.
    /// - `plan_id`: Stable semantic plan identity.
    /// - `snapshot`: Exact canonical snapshot required by the caller.
    ///
    /// Snapshot-valid rollups ordered by stable node identity, or `None` when
    /// the cache is absent, stale, or uses another evaluator version.
    ///
    /// Returns a persistence diagnostic when cached payloads are malformed.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when reading or decoding the cache fails.
    pub fn read_plan_rollup_index(
        &self,
        session_id: EntityId,
        plan_id: EntityId,
        snapshot: SnapshotId,
    ) -> Result<Option<BTreeMap<EntityId, PlanNodeRollup>>, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "session repository lock poisoned".to_owned())?;
        let rows = {
            let mut statement = connection
                .prepare(
                    "SELECT node_id, node_revision, source_snapshot, evaluator_version, payload
                     FROM plan_node_rollup_index
                     WHERE session_id=?1 AND plan_id=?2 ORDER BY node_id",
                )
                .map_err(|error| error.to_string())?;
            statement
                .query_map(
                    params![session_id.to_string(), plan_id.to_string()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, Vec<u8>>(4)?,
                        ))
                    },
                )
                .map_err(|error| error.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| error.to_string())?
        };
        if rows.is_empty()
            || rows
                .iter()
                .any(|(_, _, source, version, _)| source != &snapshot.to_string() || *version != 1)
        {
            return Ok(None);
        }
        drop(connection);
        let revisions = self
            .semantic_store
            .iter(snapshot)
            .map_err(|error| format!("plan snapshot revision read failed: {error:?}"))?
            .into_iter()
            .collect::<BTreeMap<EntityId, RevisionId>>();
        let mut rollups = BTreeMap::new();
        for (node_text, node_revision, _, _, payload) in rows {
            let node = node_text
                .parse::<EntityId>()
                .map_err(|error| format!("invalid rollup node identity: {error}"))?;
            if revisions
                .get(&node)
                .is_none_or(|revision| revision.to_string() != node_revision)
            {
                return Ok(None);
            }
            let rollup: PlanNodeRollup = serde_json::from_slice(&payload)
                .map_err(|error| format!("plan rollup decoding failed: {error}"))?;
            if rollup.node != node || rollups.insert(node, rollup).is_some() {
                return Err("plan rollup index has mismatched or duplicate nodes".to_owned());
            }
        }
        Ok(Some(rollups))
    }
}
