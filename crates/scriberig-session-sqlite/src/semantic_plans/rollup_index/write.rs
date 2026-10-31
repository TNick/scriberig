//! Writes rebuildable plan rollups and their plan-level validity metadata.

use std::collections::BTreeMap;

use rusqlite::{Transaction, params};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{PlanNodeRollup, PlanNodeStatus};

/// Replaces every rollup row when rebuilding from canonical plan state.
///
/// # Parameters
/// - `transaction`: Active plan index transaction.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact semantic snapshot represented by the rebuilt rows.
/// - `rollups`: Reference-evaluated derived state for every plan node.
///
/// # Returns
/// Unit after the complete cache and validity metadata are stored.
///
/// # Errors
/// Returns a SQLite diagnostic if any row cannot be written.
pub(crate) fn replace_in_transaction(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
    rollups: &BTreeMap<EntityId, PlanNodeRollup>,
) -> Result<(), String> {
    transaction
        .execute(
            "DELETE FROM plan_node_rollup_index WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
        )
        .map_err(|error| error.to_string())?;
    for rollup in rollups.values() {
        write_rollup_row(transaction, session, plan, snapshot, rollup)?;
    }
    write_rollup_metadata(transaction, session, plan, snapshot, rollups.len())
}

/// Inserts or updates one normalized rollup row.
///
/// # Parameters
/// - `transaction`: Active plan index transaction.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact snapshot for this row's latest materialization.
/// - `rollup`: Reference-evaluated derived state for one node.
///
/// # Returns
/// Unit after the row is stored.
///
/// # Errors
/// Returns a SQLite diagnostic when the row cannot be written.
pub(crate) fn write_rollup_row(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
    rollup: &PlanNodeRollup,
) -> Result<(), String> {
    transaction
        .execute(
            "INSERT INTO plan_node_rollup_index (
                 session_id, plan_id, node_id, effective_status,
                 contains_ignored, contains_abandoned, child_planned,
                 child_ready, child_running, child_completed, child_failed,
                 child_abandoned, unsatisfied_completion_gates,
                 entry_pending, entry_satisfied, entry_failed, entry_waived,
                 completion_pending, completion_satisfied, completion_failed,
                 completion_waived, evidence_count, evaluator_version,
                 source_snapshot
             ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                 ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22,
                 2, ?23
             ) ON CONFLICT(session_id, plan_id, node_id) DO UPDATE SET
                 effective_status=excluded.effective_status,
                 contains_ignored=excluded.contains_ignored,
                 contains_abandoned=excluded.contains_abandoned,
                 child_planned=excluded.child_planned,
                 child_ready=excluded.child_ready,
                 child_running=excluded.child_running,
                 child_completed=excluded.child_completed,
                 child_failed=excluded.child_failed,
                 child_abandoned=excluded.child_abandoned,
                 unsatisfied_completion_gates=excluded.unsatisfied_completion_gates,
                 entry_pending=excluded.entry_pending,
                 entry_satisfied=excluded.entry_satisfied,
                 entry_failed=excluded.entry_failed,
                 entry_waived=excluded.entry_waived,
                 completion_pending=excluded.completion_pending,
                 completion_satisfied=excluded.completion_satisfied,
                 completion_failed=excluded.completion_failed,
                 completion_waived=excluded.completion_waived,
                 evidence_count=excluded.evidence_count,
                 evaluator_version=excluded.evaluator_version,
                 source_snapshot=excluded.source_snapshot",
            params![
                session.as_bytes().as_slice(),
                plan.as_bytes().as_slice(),
                rollup.node.as_bytes().as_slice(),
                status_tag(rollup.effective_status),
                rollup.contains_ignored,
                rollup.contains_abandoned,
                i64::from(rollup.child_counts.planned),
                i64::from(rollup.child_counts.ready),
                i64::from(rollup.child_counts.running),
                i64::from(rollup.child_counts.completed),
                i64::from(rollup.child_counts.failed),
                i64::from(rollup.child_counts.abandoned),
                i64::from(rollup.unsatisfied_completion_gates),
                i64::from(rollup.entry_gates.pending),
                i64::from(rollup.entry_gates.satisfied),
                i64::from(rollup.entry_gates.failed),
                i64::from(rollup.entry_gates.waived),
                i64::from(rollup.completion_gates.pending),
                i64::from(rollup.completion_gates.satisfied),
                i64::from(rollup.completion_gates.failed),
                i64::from(rollup.completion_gates.waived),
                i64::from(rollup.evidence_count),
                snapshot.as_bytes().as_slice(),
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Advances exact-snapshot validity metadata for a complete normalized cache.
///
/// # Parameters
/// - `transaction`: Active plan index transaction.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact semantic snapshot represented by the cache.
/// - `node_count`: Expected number of direct nodes and derived rows.
///
/// # Returns
/// Unit after plan-level cache metadata is current.
///
/// # Errors
/// Returns a SQLite diagnostic when metadata cannot be written.
pub(crate) fn write_rollup_metadata(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
    node_count: usize,
) -> Result<(), String> {
    transaction
        .execute(
            "INSERT INTO plan_node_rollup_meta (
                 session_id, plan_id, source_snapshot, evaluator_version, node_count
             ) VALUES (?1, ?2, ?3, 2, ?4)
             ON CONFLICT(session_id, plan_id) DO UPDATE SET
                 source_snapshot=excluded.source_snapshot,
                 evaluator_version=excluded.evaluator_version,
                 node_count=excluded.node_count",
            params![
                session.as_bytes().as_slice(),
                plan.as_bytes().as_slice(),
                snapshot.as_bytes().as_slice(),
                i64::try_from(node_count)
                    .map_err(|_| "plan rollup node count exceeds SQLite range".to_owned())?,
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Encodes one lifecycle value as its stable database label.
///
/// # Parameters
/// - `status`: Canonical plan-node lifecycle value.
///
/// # Returns
/// The normalized lowercase status label.
fn status_tag(status: PlanNodeStatus) -> &'static str {
    match status {
        PlanNodeStatus::Planned => "planned",
        PlanNodeStatus::Ready => "ready",
        PlanNodeStatus::Running => "running",
        PlanNodeStatus::Completed => "completed",
        PlanNodeStatus::Failed => "failed",
        PlanNodeStatus::Abandoned => "abandoned",
    }
}
