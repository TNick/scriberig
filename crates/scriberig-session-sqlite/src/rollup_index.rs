//! Persists rebuildable plan rollups outside canonical semantic revisions.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{TransactionBehavior, params};
use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_session::{Plan, PlanNodeRollup, PlanNodeRollupIndex, SessionStore};
use scriberig_store::SnapshotStore;

use crate::SqliteSessionStore;

impl SqliteSessionStore {
    /// Propagates changed plan rollups and advances their snapshot binding.
    ///
    /// A missing or stale prior cache is rebuilt from the new canonical plan.
    /// Node membership changes use the same full rebuild path because removal
    /// is not supported by the incremental index API.
    ///
    /// # Parameters
    /// - `self`: SQLite session repository owning the derived index.
    /// - `session_id`: Owning `ScribeRig` session identity.
    /// - `previous`: Plan projection from before the successful semantic CAS.
    /// - `current`: Plan projection loaded after the successful semantic CAS.
    ///
    /// # Returns
    /// Unit after updating the derived cache or rebuilding it.
    ///
    /// # Errors
    /// Returns a persistence diagnostic if cache reading or updating fails.
    pub(super) fn update_plan_rollup_index_after_cas(
        &self,
        session_id: EntityId,
        previous: &Plan,
        current: &Plan,
    ) -> Result<(), String> {
        let old_snapshot = previous
            .source_snapshot
            .ok_or_else(|| "prior plan has no semantic snapshot".to_owned())?;
        let new_snapshot = current
            .source_snapshot
            .ok_or_else(|| "current plan has no semantic snapshot".to_owned())?;
        let old_nodes = previous
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<BTreeSet<_>>();
        let new_nodes = current
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<BTreeSet<_>>();
        if old_nodes != new_nodes {
            self.rebuild_plan_rollup_index(session_id, current.id)?;
            return Ok(());
        }
        let Ok(Some(cached)) = self.read_plan_rollup_index(session_id, current.id, old_snapshot)
        else {
            self.rebuild_plan_rollup_index(session_id, current.id)?;
            return Ok(());
        };
        let Ok(mut index) =
            PlanNodeRollupIndex::from_cached_rollups(previous, old_snapshot, cached)
        else {
            self.rebuild_plan_rollup_index(session_id, current.id)?;
            return Ok(());
        };
        let mut changed = previous
            .nodes
            .iter()
            .filter_map(|old| {
                current
                    .nodes
                    .iter()
                    .find(|new| new.id == old.id)
                    .filter(|new| *new != old)
                    .map(|new| new.id)
            })
            .collect::<BTreeSet<_>>();
        if previous.gates != current.gates || previous.evidence != current.evidence {
            changed.extend(previous.gates.iter().map(|gate| gate.owner));
            changed.extend(current.gates.iter().map(|gate| gate.owner));
        }
        let Ok(touched) = index.apply_change(
            current,
            old_snapshot,
            new_snapshot,
            &changed.iter().copied().collect::<Vec<_>>(),
        ) else {
            self.rebuild_plan_rollup_index(session_id, current.id)?;
            return Ok(());
        };
        self.persist_incremental_plan_rollups(
            session_id,
            current,
            &index,
            &changed.into_iter().chain(touched).collect(),
        )
    }

    /// Persists changed rollups while refreshing cache snapshot provenance.
    ///
    /// # Parameters
    /// - `self`: SQLite session repository owning the derived index.
    /// - `session_id`: Owning `ScribeRig` session identity.
    /// - `plan`: Current canonical plan projection.
    /// - `index`: Incrementally updated rollup index.
    /// - `changed`: Node rows requiring status, revision, or parent refresh.
    ///
    /// # Returns
    /// Unit after the cache transaction commits.
    ///
    /// # Errors
    /// Returns a persistence diagnostic when the derived rows cannot be stored.
    fn persist_incremental_plan_rollups(
        &self,
        session_id: EntityId,
        plan: &Plan,
        index: &PlanNodeRollupIndex,
        changed: &BTreeSet<EntityId>,
    ) -> Result<(), String> {
        let snapshot = index.source_snapshot();
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
                "UPDATE plan_node_rollup_index SET source_snapshot=?3
                 WHERE session_id=?1 AND plan_id=?2",
                params![
                    session_id.to_string(),
                    plan.id.to_string(),
                    snapshot.to_string()
                ],
            )
            .map_err(|error| error.to_string())?;
        for node_id in changed {
            let Some(rollup) = index.get(*node_id) else {
                continue;
            };
            let node = plan
                .nodes
                .iter()
                .find(|node| node.id == *node_id)
                .ok_or_else(|| "incremental rollup references a missing node".to_owned())?;
            let parent = match node.parent {
                Some(scriberig_session::PlanNodeParent::Node(parent)) => Some(parent.to_string()),
                Some(scriberig_session::PlanNodeParent::Branch(_)) | None => None,
            };
            let revision = revisions
                .get(node_id)
                .ok_or_else(|| "plan node is absent from its source snapshot".to_owned())?;
            let payload = serde_json::to_vec(rollup)
                .map_err(|error| format!("plan rollup encoding failed: {error}"))?;
            let updated = transaction
                .execute(
                    "UPDATE plan_node_rollup_index SET parent_node_id=?4,
                         node_revision=?5, direct_status=?6, effective_status=?7,
                         evaluator_version=?8, payload=?9
                     WHERE session_id=?1 AND plan_id=?2 AND node_id=?3",
                    params![
                        session_id.to_string(),
                        plan.id.to_string(),
                        node_id.to_string(),
                        parent,
                        revision.to_string(),
                        format!("{:?}", rollup.direct_status),
                        format!("{:?}", rollup.effective_status),
                        i64::from(index.evaluator_version()),
                        payload,
                    ],
                )
                .map_err(|error| error.to_string())?;
            if updated == 0 {
                transaction
                    .execute(
                        "INSERT INTO plan_node_rollup_index (
                             session_id, plan_id, node_id, parent_node_id,
                             node_revision, direct_status, effective_status,
                             source_snapshot, evaluator_version, payload
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        params![
                            session_id.to_string(),
                            plan.id.to_string(),
                            node_id.to_string(),
                            parent,
                            revision.to_string(),
                            format!("{:?}", rollup.direct_status),
                            format!("{:?}", rollup.effective_status),
                            snapshot.to_string(),
                            i64::from(index.evaluator_version()),
                            payload,
                        ],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        transaction.commit().map_err(|error| error.to_string())
    }

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
                .any(|(_, _, source, version, _)| source != &snapshot.to_string() || *version != 2)
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
