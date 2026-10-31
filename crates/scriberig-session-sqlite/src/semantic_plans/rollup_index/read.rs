//! Reads and validates normalized plan rollup rows from SQLite.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{PlanGateCounts, PlanNodeRollup, PlanNodeStatus, PlanNodeStatusCounts};

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
pub(super) fn read_rollup_rows(
    connection: &Connection,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
) -> Result<Option<BTreeMap<EntityId, PlanNodeRollup>>, String> {
    let Some((source_snapshot, evaluator_version, node_count)) =
        read_rollup_metadata(connection, session, plan)?
    else {
        return Ok(None);
    };
    if source_snapshot.as_slice() != snapshot.as_bytes()
        || evaluator_version != 2
        || !node_index_matches_snapshot(connection, session, plan, snapshot, node_count)?
    {
        return Ok(None);
    }
    let rows = read_rollup_values(connection, session, plan)?;
    if !rollup_row_counts_match(connection, session, plan, node_count, rows.len())? {
        return Ok(None);
    }
    decode_rollups(rows)
}

/// Reads plan-level validity metadata for one rollup cache.
///
/// # Parameters
/// - `connection`: SQLite connection guarded by the session repository.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
///
/// # Returns
/// Snapshot bytes, evaluator version, and node count when metadata exists.
///
/// # Errors
/// Returns a diagnostic when SQLite cannot read the metadata.
fn read_rollup_metadata(
    connection: &Connection,
    session: EntityId,
    plan: EntityId,
) -> Result<Option<(Vec<u8>, i64, i64)>, String> {
    connection
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
        .map_err(|error| error.to_string())
}

/// Reads all normalized rollup values for one plan identity.
///
/// # Parameters
/// - `connection`: SQLite connection guarded by the session repository.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
///
/// # Returns
/// Joined direct-node and rollup records ordered by node identity.
///
/// # Errors
/// Returns a diagnostic when rows cannot be queried or decoded.
fn read_rollup_values(
    connection: &Connection,
    session: EntityId,
    plan: EntityId,
) -> Result<Vec<RollupRow>, String> {
    let mut statement = connection
        .prepare(
            "SELECT rollup.node_id, node.direct_status, rollup.effective_status,
                    rollup.contains_ignored, rollup.contains_abandoned,
                    rollup.child_planned, rollup.child_ready, rollup.child_running,
                    rollup.child_completed, rollup.child_failed,
                    rollup.child_abandoned, rollup.unsatisfied_completion_gates,
                    rollup.entry_pending, rollup.entry_satisfied,
                    rollup.entry_failed, rollup.entry_waived,
                    rollup.completion_pending, rollup.completion_satisfied,
                    rollup.completion_failed, rollup.completion_waived,
                    rollup.evidence_count, rollup.evaluator_version
             FROM plan_node_rollup_index AS rollup
             JOIN plan_node_index AS node
               ON node.session_id=rollup.session_id
              AND node.plan_id=rollup.plan_id AND node.node_id=rollup.node_id
             WHERE rollup.session_id=?1 AND rollup.plan_id=?2
             ORDER BY rollup.node_id",
        )
        .map_err(|error| error.to_string())?;
    statement
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
                    [
                        row.get::<_, i64>(12)?,
                        row.get::<_, i64>(13)?,
                        row.get::<_, i64>(14)?,
                        row.get::<_, i64>(15)?,
                    ],
                    [
                        row.get::<_, i64>(16)?,
                        row.get::<_, i64>(17)?,
                        row.get::<_, i64>(18)?,
                        row.get::<_, i64>(19)?,
                    ],
                    row.get::<_, i64>(20)?,
                    row.get::<_, i64>(21)?,
                ))
            },
        )
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

/// Checks that direct and rollup row counts match cache metadata.
///
/// # Parameters
/// - `connection`: SQLite connection guarded by the session repository.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `expected`: Node count declared by plan-level metadata.
/// - `loaded`: Number of joined rollup rows read.
///
/// # Returns
/// `true` when both indexed tables contain the expected number of rows.
///
/// # Errors
/// Returns a diagnostic when either count query fails.
fn rollup_row_counts_match(
    connection: &Connection,
    session: EntityId,
    plan: EntityId,
    expected: i64,
    loaded: usize,
) -> Result<bool, String> {
    let direct: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM plan_node_index
             WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let rollups: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM plan_node_rollup_index
             WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    Ok(usize::try_from(expected).ok() == Some(loaded) && direct == expected && rollups == expected)
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
        entry_gates,
        completion_gates,
        evidence_count,
        evaluator_version,
    ) in rows
    {
        if evaluator_version != 2 {
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
                entry_gates: decode_gate_counts(entry_gates)?,
                completion_gates: decode_gate_counts(completion_gates)?,
                evidence_count: decode_count(evidence_count)?,
            },
        );
    }
    Ok(Some(rollups))
}

/// Holds the SQLite columns required to reconstruct one rollup.
type RollupRow = (
    Vec<u8>,
    String,
    String,
    bool,
    bool,
    [i64; 6],
    i64,
    [i64; 4],
    [i64; 4],
    i64,
    i64,
);

/// Decodes four gate-state counts from their normalized database columns.
///
/// # Parameters
/// - `values`: Pending, satisfied, failed, and waived counts.
///
/// # Returns
/// The validated gate counts.
///
/// # Errors
/// Returns a diagnostic for a negative or overflowing stored count.
fn decode_gate_counts(values: [i64; 4]) -> Result<PlanGateCounts, String> {
    Ok(PlanGateCounts {
        pending: decode_count(values[0])?,
        satisfied: decode_count(values[1])?,
        failed: decode_count(values[2])?,
        waived: decode_count(values[3])?,
    })
}

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
