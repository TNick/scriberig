//! Maintains rebuildable normalized plan-node indexes in SQLite.

use rusqlite::{Transaction, params};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{PlanNodeParent, PlanNodeStatus, PlanSemanticNodeRef};
use scriberig_store::RootStore;

use super::plan_root;
use crate::SqliteSessionStore;

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
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
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
    }
    Ok(())
}

/// Replaces direct and derived node index rows in one root-promotion transaction.
///
/// # Parameters
/// - `transaction`: Transaction promoting the plan root.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact successor snapshot represented by both indexes.
/// - `nodes`: Revision-bound canonical node rows.
/// - `rollups`: Reference-evaluated derived rollup rows.
///
/// # Returns
/// Unit after both indexes identify the successor snapshot.
///
/// # Errors
/// Returns a diagnostic from either index replacement operation.
pub(super) fn replace_indexes_in_transaction(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
    nodes: &[PlanSemanticNodeRef],
    rollups: &std::collections::BTreeMap<EntityId, scriberig_session::PlanNodeRollup>,
) -> Result<(), String> {
    replace_in_transaction(transaction, session, plan, snapshot, nodes)?;
    super::rollup_index::replace_in_transaction(transaction, session, plan, snapshot, rollups)
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
