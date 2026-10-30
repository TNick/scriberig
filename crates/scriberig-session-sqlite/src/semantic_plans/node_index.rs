//! Maintains rebuildable normalized plan-node indexes in SQLite.

use rusqlite::{Transaction, params};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{PlanNodeParent, PlanNodeStatus, PlanSemanticNodeRef};
use scriberig_store::RootStore;

use super::plan_root;
use crate::SqliteSessionStore;

#[path = "node_index/update.rs"]
mod update;
pub(super) use update::{persist_in_transaction, stage_update};

impl SqliteSessionStore {
    /// Rebuilds the normalized node index for the current plan root.
    ///
    /// # Parameters
    /// - `self`: SQLite session repository.
    /// - `session`: Owning durable session identity.
    /// - `plan`: Stable plan identity.
    ///
    /// # Returns
    /// Unit after index rows identify the current exact source snapshot.
    ///
    /// # Errors
    /// Returns a diagnostic when canonical plan data or the index transaction
    /// cannot be read or written.
    pub fn rebuild_plan_node_index(&self, session: EntityId, plan: EntityId) -> Result<(), String> {
        rebuild_plan_node_index(self, session, plan)
    }
}

/// Rebuilds normalized node-index rows from the current canonical plan root.
///
/// # Parameters
/// - `store`: SQLite session repository.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
///
/// # Returns
/// Unit after index rows identify the current exact source snapshot.
///
/// # Errors
/// Returns a diagnostic when the root, semantic plan, or SQLite transaction
/// cannot be read or written.
pub(super) fn rebuild_plan_node_index(
    store: &SqliteSessionStore,
    session: EntityId,
    plan: EntityId,
) -> Result<(), String> {
    let root = plan_root(session, plan)?;
    let snapshot = store
        .semantic_store
        .read_root(&root)
        .ok_or_else(|| "plan semantic root is absent".to_owned())?;
    let projection = store.load_plan_at_snapshot(session, plan, snapshot)?;
    let semantic = scriberig_session::PlanSemanticSnapshot::from_plan(&projection)
        .map_err(|error| format!("plan semantic index encode failed: {error:?}"))?;
    let nodes = semantic
        .node_refs()
        .map_err(|error| format!("plan node index decode failed: {error:?}"))?;
    let rollups = projection
        .node_rollups()
        .map_err(|error| format!("plan rollup index rebuild failed: {error:?}"))?;
    let mut connection = store
        .connection
        .lock()
        .map_err(|_| "session repository lock poisoned".to_owned())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    replace_in_transaction(&transaction, session, plan, snapshot, &nodes)?;
    super::rollup_index::replace_in_transaction(&transaction, session, plan, snapshot, &rollups)?;
    transaction.commit().map_err(|error| error.to_string())
}

/// Replaces normalized node rows inside the plan-promotion transaction.
///
/// # Parameters
/// - `transaction`: SQLite transaction already promoting the plan root.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact successor snapshot represented by these rows.
/// - `nodes`: Revision-bound nodes projected from the proposed semantic plan.
///
/// # Returns
/// Unit after all node rows have been replaced.
///
/// # Errors
/// Returns a SQLite diagnostic when rows cannot be replaced.
pub(super) fn replace_in_transaction(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
    nodes: &[PlanSemanticNodeRef],
) -> Result<(), String> {
    transaction
        .execute(
            "DELETE FROM plan_node_index WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
        )
        .map_err(|error| error.to_string())?;
    for node in nodes {
        write_node_row(transaction, session, plan, snapshot, node)?;
    }
    write_index_metadata(transaction, session, plan, snapshot, nodes.len())
}

/// Inserts or replaces one normalized direct-node row.
///
/// # Parameters
/// - `transaction`: Active plan index transaction.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact snapshot represented by this changed row.
/// - `node`: Canonical revision and structural position.
///
/// # Returns
/// Unit after the row is stored.
///
/// # Errors
/// Returns a SQLite diagnostic when the row cannot be written.
fn write_node_row(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
    node: &PlanSemanticNodeRef,
) -> Result<(), String> {
    let (parent_node, parent_branch) = match node.parent {
        Some(PlanNodeParent::Node(parent)) => (Some(parent), None),
        Some(PlanNodeParent::Branch(parent)) => (None, Some(parent)),
        None => (None, None),
    };
    transaction
        .execute(
            "INSERT INTO plan_node_index (
                 session_id, plan_id, node_id, parent_node_id, parent_branch_id,
                 current_revision_id, direct_status, source_snapshot
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(session_id, plan_id, node_id) DO UPDATE SET
                 parent_node_id=excluded.parent_node_id,
                 parent_branch_id=excluded.parent_branch_id,
                 current_revision_id=excluded.current_revision_id,
                 direct_status=excluded.direct_status,
                 source_snapshot=excluded.source_snapshot",
            params![
                session.as_bytes().as_slice(),
                plan.as_bytes().as_slice(),
                node.entity.as_bytes().as_slice(),
                parent_node.map(|id| id.as_bytes().to_vec()),
                parent_branch.map(|id| id.as_bytes().to_vec()),
                node.revision.as_bytes().as_slice(),
                status_tag(node.status),
                snapshot.as_bytes().as_slice(),
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Advances plan-level node-index snapshot and completeness metadata.
///
/// # Parameters
/// - `transaction`: Active plan index transaction.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact snapshot represented by the complete index.
/// - `node_count`: Expected membership row count.
///
/// # Returns
/// Unit after metadata is current.
///
/// # Errors
/// Returns a SQLite diagnostic when metadata cannot be written.
fn write_index_metadata(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
    node_count: usize,
) -> Result<(), String> {
    transaction
        .execute(
            "INSERT INTO plan_node_index_meta (session_id, plan_id, source_snapshot, node_count)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id, plan_id) DO UPDATE SET
                 source_snapshot=excluded.source_snapshot, node_count=excluded.node_count",
            params![
                session.as_bytes().as_slice(),
                plan.as_bytes().as_slice(),
                snapshot.as_bytes().as_slice(),
                i64::try_from(node_count)
                    .map_err(|_| "plan node count exceeds SQLite range".to_owned())?,
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Counts direct-node rows for one plan in the active transaction.
///
/// # Parameters
/// - `transaction`: Active SQLite transaction.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
///
/// # Returns
/// Number of indexed node rows.
///
/// # Errors
/// Returns a SQLite diagnostic when the count cannot be read.
fn index_node_count(
    transaction: &rusqlite::Connection,
    session: EntityId,
    plan: EntityId,
) -> Result<usize, String> {
    let count: i64 = transaction
        .query_row(
            "SELECT COUNT(*) FROM plan_node_index WHERE session_id=?1 AND plan_id=?2",
            params![session.as_bytes().as_slice(), plan.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    usize::try_from(count).map_err(|_| "negative plan node index count".to_owned())
}

/// Returns the stable database label for one canonical node status.
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
