//! Stages sparse direct-node index updates from semantic plan revisions.

use rusqlite::{OptionalExtension, Transaction, params};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_session::{Plan, PlanSemanticNodeRef};

use crate::SqliteSessionStore;

/// Holds direct-node rows changed by one plan-root promotion.
///
/// # Public members
/// None.
///
/// # Private members
/// - `snapshot`: Successor snapshot represented by the complete node index.
/// - `changed`: Node rows to insert or replace.
/// - `removed`: Node identities removed from membership.
/// - `replace_all`: Whether the previous normalized cache was not current.
pub struct StagedNodeIndex {
    pub(super) snapshot: SnapshotId,
    changed: Vec<PlanSemanticNodeRef>,
    removed: Vec<EntityId>,
    replace_all: bool,
}

/// Stages sparse direct-node rows from the previous canonical plan.
///
/// # Parameters
/// - `store`: SQLite session repository owning the node-index cache.
/// - `session`: Owning durable session identity.
/// - `previous`: Canonical plan at the base snapshot.
/// - `proposed`: Canonical plan being promoted.
/// - `old_snapshot`: Exact base snapshot.
/// - `new_snapshot`: Exact successor snapshot.
/// - `new_nodes`: Revision-bound node refs encoded from the proposed plan.
///
/// # Returns
/// Sparse changed and removed rows, or a full-replacement update when the
/// prior cache is absent, incomplete, or stale.
///
/// # Errors
/// Returns a diagnostic when prior node refs cannot be encoded or SQLite
/// validity metadata cannot be read.
pub fn stage_update(
    store: &SqliteSessionStore,
    session: EntityId,
    previous: &Plan,
    proposed: &Plan,
    old_snapshot: SnapshotId,
    new_snapshot: SnapshotId,
    new_nodes: Vec<PlanSemanticNodeRef>,
) -> Result<StagedNodeIndex, String> {
    let previous_nodes = scriberig_session::PlanSemanticSnapshot::from_plan(previous)
        .and_then(|snapshot| snapshot.node_refs())
        .map_err(|error| format!("prior plan node index encode failed: {error:?}"))?;
    if !has_current_index(
        store,
        session,
        proposed.id,
        old_snapshot,
        previous_nodes.len(),
    )? {
        return Ok(StagedNodeIndex {
            snapshot: new_snapshot,
            changed: new_nodes,
            removed: Vec::new(),
            replace_all: true,
        });
    }
    let old_by_id = previous_nodes
        .iter()
        .map(|node| (node.entity, node))
        .collect::<std::collections::BTreeMap<_, _>>();
    let new_ids = new_nodes
        .iter()
        .map(|node| node.entity)
        .collect::<std::collections::BTreeSet<_>>();
    let changed = new_nodes
        .into_iter()
        .filter(|node| old_by_id.get(&node.entity).is_none_or(|old| **old != *node))
        .collect();
    let removed = old_by_id
        .keys()
        .filter(|node| !new_ids.contains(node))
        .copied()
        .collect();
    Ok(StagedNodeIndex {
        snapshot: new_snapshot,
        changed,
        removed,
        replace_all: false,
    })
}

/// Persists a staged node index inside the root-promotion transaction.
///
/// # Parameters
/// - `transaction`: Transaction already promoting the plan root.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `staged`: Changed rows and exact successor index metadata.
///
/// # Returns
/// Unit after changed rows and validity metadata are committed atomically.
///
/// # Errors
/// Returns a SQLite diagnostic when a row or metadata update fails.
pub fn persist_in_transaction(
    transaction: &Transaction<'_>,
    session: EntityId,
    plan: EntityId,
    staged: &StagedNodeIndex,
) -> Result<(), String> {
    if staged.replace_all {
        super::replace_in_transaction(
            transaction,
            session,
            plan,
            staged.snapshot,
            &staged.changed,
        )?;
        return Ok(());
    }
    for node in &staged.removed {
        transaction
            .execute(
                "DELETE FROM plan_node_index WHERE session_id=?1 AND plan_id=?2 AND node_id=?3",
                params![
                    session.as_bytes().as_slice(),
                    plan.as_bytes().as_slice(),
                    node.as_bytes().as_slice(),
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    for node in &staged.changed {
        super::write_node_row(transaction, session, plan, staged.snapshot, node)?;
    }
    let count = super::index_node_count(transaction, session, plan)?;
    super::write_index_metadata(transaction, session, plan, staged.snapshot, count)
}

/// Reports whether cached rows and metadata identify one complete prior snapshot.
///
/// # Parameters
/// - `store`: SQLite session repository containing the index.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable plan identity.
/// - `snapshot`: Exact base snapshot expected by the caller.
/// - `expected_count`: Number of canonical node refs in that snapshot.
///
/// # Returns
/// `true` when metadata and row count match the exact prior index.
///
/// # Errors
/// Returns a diagnostic when SQLite cannot read index metadata.
fn has_current_index(
    store: &SqliteSessionStore,
    session: EntityId,
    plan: EntityId,
    snapshot: SnapshotId,
    expected_count: usize,
) -> Result<bool, String> {
    let connection = store
        .connection
        .lock()
        .map_err(|_| "session repository lock poisoned".to_owned())?;
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
    if source_snapshot.as_slice() != snapshot.as_bytes()
        || usize::try_from(node_count).ok() != Some(expected_count)
    {
        return Ok(false);
    }
    Ok(super::index_node_count(&connection, session, plan)? == expected_count)
}
