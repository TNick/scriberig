//! Persists rebuildable effective plan-node rollups in SQLite.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{PlanNodeRollup, PlanNodeStatus, PlanNodeStatusCounts};
use scriberig_store::RootStore;

use super::plan_root;
use crate::SqliteSessionStore;

#[path = "rollup_index/write.rs"]
mod write;
pub(super) use write::{replace_in_transaction, write_rollup_metadata, write_rollup_row};

impl SqliteSessionStore {
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

/// Reads and validates normalized rows for one plan identity.
///
/// # Parameters
/// - `connection`: SQLite connection guarded by the session repository.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact snapshot expected in both index tables.
///
/// # Returns
/// Complete cached rollups, or `None` when rows are missing or stale.
///
/// # Errors
/// Returns a diagnostic when normalized rows are malformed or unreadable.
fn read_rollup_rows(
    connection: &Connection,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
) -> Result<Option<BTreeMap<EntityId, PlanNodeRollup>>, String> {
    let metadata = connection
        .query_row(
            "SELECT source_snapshot, evaluator_version, node_count
             FROM plan_node_rollup_meta WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((source_snapshot, evaluator_version, node_count)) = metadata else {
        return Ok(None);
    };
    if source_snapshot.as_slice() != snapshot.as_bytes() || evaluator_version != 1 {
        return Ok(None);
    }
    if !node_index_matches_snapshot(connection, session, plan, snapshot, node_count)? {
        return Ok(None);
    }
    let mut statement = connection
        .prepare(
            "SELECT rollup.node_id, node.direct_status, rollup.effective_status,
                    rollup.contains_ignored, rollup.contains_abandoned,
                    rollup.child_planned, rollup.child_ready, rollup.child_running,
                    rollup.child_completed, rollup.child_failed,
                    rollup.child_abandoned, rollup.unsatisfied_completion_gates,
                    rollup.evaluator_version
             FROM plan_node_rollup_index AS rollup
             JOIN plan_node_index AS node
               ON node.session_id=rollup.session_id
              AND node.plan_id=rollup.plan_id AND node.node_id=rollup.node_id
             WHERE rollup.session_id=?1 AND rollup.plan_id=?2
             ORDER BY rollup.node_id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, bool>(4)?,
                    [
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, i64>(9)?,
                        row.get::<_, i64>(10)?,
                    ],
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    let direct_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM plan_node_index
             WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let rollup_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM plan_node_rollup_index
             WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if usize::try_from(node_count).ok() != Some(rows.len())
        || direct_count != node_count
        || rollup_count != node_count
    {
        return Ok(None);
    }
    decode_rollups(rows)
}

/// Checks whether direct-node index metadata matches the requested snapshot.
///
/// # Parameters
/// - `connection`: SQLite connection guarded by the session repository.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact snapshot requested by the caller.
/// - `expected_count`: Node count declared by rollup metadata.
///
/// # Returns
/// `true` when direct-index validity metadata matches the exact rollup cache.
///
/// # Errors
/// Returns a diagnostic when SQLite cannot read direct-index metadata.
fn node_index_matches_snapshot(
    connection: &Connection,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
    expected_count: i64,
) -> Result<bool, String> {
    let metadata = connection
        .query_row(
            "SELECT source_snapshot, node_count FROM plan_node_index_meta
             WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let Some((source_snapshot, node_count)) = metadata else {
        return Ok(false);
    };
    Ok(source_snapshot.as_slice() == snapshot.as_bytes() && node_count == expected_count)
}

/// Converts validated SQLite records into exact-snapshot rollup values.
///
/// # Parameters
/// - `rows`: Normalized node and rollup columns ordered by node identity.
/// # Returns
/// All joined rollups. Snapshot validity is checked by plan-level metadata.
///
/// # Errors
/// Returns a diagnostic when a row contains invalid status, identity, or count.
fn decode_rollups(
    rows: Vec<RollupRow>,
) -> Result<Option<BTreeMap<EntityId, PlanNodeRollup>>, String> {
    let mut rollups = BTreeMap::new();
    for (
        id,
        direct_status,
        effective_status,
        contains_ignored,
        contains_abandoned,
        child_counts,
        unsatisfied_gates,
        evaluator_version,
    ) in rows
    {
        if evaluator_version != 1 {
            return Ok(None);
        }
        let id = decode_entity_id(&id)?;
        rollups.insert(
            id,
            PlanNodeRollup {
                node: id,
                direct_status: parse_status(&direct_status)?,
                effective_status: parse_status(&effective_status)?,
                contains_ignored,
                contains_abandoned,
                child_counts: PlanNodeStatusCounts {
                    planned: decode_count(child_counts[0])?,
                    ready: decode_count(child_counts[1])?,
                    running: decode_count(child_counts[2])?,
                    completed: decode_count(child_counts[3])?,
                    failed: decode_count(child_counts[4])?,
                    abandoned: decode_count(child_counts[5])?,
                },
                unsatisfied_completion_gates: decode_count(unsatisfied_gates)?,
            },
        );
    }
    Ok(Some(rollups))
}

/// Holds the SQLite columns required to reconstruct one rollup.
type RollupRow = (Vec<u8>, String, String, bool, bool, [i64; 6], i64, i64);

/// Decodes a fixed-width entity identifier from SQLite.
///
/// # Parameters
/// - `bytes`: Encoded 16-byte entity identity.
///
/// # Returns
/// The exact entity identity.
///
/// # Errors
/// Returns a diagnostic when the stored identifier has an invalid width.
fn decode_entity_id(bytes: &[u8]) -> Result<EntityId, String> {
    let bytes: [u8; 16] = bytes
        .try_into()
        .map_err(|_| "invalid entity identity in plan rollup index".to_owned())?;
    Ok(EntityId::from_bytes(bytes))
}

/// Decodes a nonnegative SQLite count into the plan's bounded count type.
///
/// # Parameters
/// - `value`: Stored count value.
///
/// # Returns
/// The count represented by the normalized row.
///
/// # Errors
/// Returns a diagnostic for negative or overflowing stored counts.
fn decode_count(value: i64) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| "invalid count in plan rollup index".to_owned())
}

/// Decodes one normalized lifecycle label.
///
/// # Parameters
/// - `value`: Lowercase database status label.
///
/// # Returns
/// The matching canonical plan-node status.
///
/// # Errors
/// Returns a diagnostic for an unknown persisted status.
fn parse_status(value: &str) -> Result<PlanNodeStatus, String> {
    match value {
        "planned" => Ok(PlanNodeStatus::Planned),
        "ready" => Ok(PlanNodeStatus::Ready),
        "running" => Ok(PlanNodeStatus::Running),
        "completed" => Ok(PlanNodeStatus::Completed),
        "failed" => Ok(PlanNodeStatus::Failed),
        "abandoned" => Ok(PlanNodeStatus::Abandoned),
        _ => Err("invalid status in plan rollup index".to_owned()),
    }
}
