//! Persists rebuildable effective plan-node rollups in SQLite.

use std::collections::BTreeMap;

use rusqlite::{Transaction, params};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{PlanNodeRollup, PlanNodeStatus};

/// Replaces normalized rollup rows inside the plan-promotion transaction.
///
/// # Parameters
/// - `transaction`: Transaction promoting the canonical plan root.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact successor snapshot represented by these rows.
/// - `rollups`: Reference-evaluated derived state for every plan node.
///
/// # Returns
/// Unit after every rollup row has been replaced.
///
/// # Errors
/// Returns a SQLite diagnostic if any row cannot be written.
pub(super) fn replace_in_transaction(
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
        transaction
            .execute(
                "INSERT INTO plan_node_rollup_index (
                     session_id, plan_id, node_id, effective_status,
                     contains_ignored, contains_abandoned, child_planned,
                     child_ready, child_running, child_completed, child_failed,
                     child_abandoned, unsatisfied_completion_gates,
                     evaluator_version, source_snapshot
                 ) VALUES (
                     ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                     ?13, 1, ?14
                 )",
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
                    snapshot.as_bytes().as_slice(),
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Returns the stable database label for one canonical lifecycle status.
///
/// # Parameters
/// - `status`: Canonical plan-node status to encode.
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
