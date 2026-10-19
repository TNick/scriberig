//! Marks canonical SQLite objects reachable from a stable protected-root set.

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, params};
use scriberig_core::RevisionId;
use scriberig_store::{ProtectedRoots, ReachableObjects, Snapshot, SnapshotNodeId};

use crate::snapshot_integrity;

/// Marks persistent canonical objects reachable from protected snapshots.
///
/// - `connection`: SQLite connection containing canonical storage.
/// - `protected`: Stable protected-root set captured by the caller.
/// - `crate`: Internal visibility marker.
///
/// Reachable snapshots, revisions, and persistent map nodes.
///
/// Returns an error when canonical storage cannot be traversed.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn mark(
    connection: &Connection,
    protected: &ProtectedRoots,
) -> rusqlite::Result<ReachableObjects> {
    let mut reachable = ReachableObjects::default();
    for snapshot_id in &protected.snapshots {
        if *snapshot_id == Snapshot::empty().id() {
            reachable.snapshots.insert(*snapshot_id);
            continue;
        }
        let root: Option<Vec<u8>> = connection
            .query_row(
                "SELECT node_id FROM snapshot_node_roots WHERE snapshot_id=?",
                params![snapshot_id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(root) = root {
            let walk = snapshot_integrity::inspect_nodes(connection, &root).map_err(|error| {
                rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(format!(
                    "protected snapshot traversal failed: {error:?}"
                ))))
            })?;
            reachable.snapshots.insert(*snapshot_id);
            for node in walk.nodes {
                reachable.snapshot_nodes.insert(node);
            }
            for revision in walk.entries.values() {
                reachable.revisions.insert(*revision);
            }
            continue;
        }
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM snapshots WHERE id=?)",
            params![snapshot_id.as_bytes().as_slice()],
            |row| row.get(0),
        )?;
        if exists {
            reachable.snapshots.insert(*snapshot_id);
            let mut rows =
                connection.prepare("SELECT revision FROM snapshot_entries WHERE snapshot_id=?")?;
            let revisions = rows.query_map(params![snapshot_id.as_bytes().as_slice()], |row| {
                row.get::<_, Vec<u8>>(0)
            })?;
            for revision in revisions {
                let revision = revision?;
                if let Ok(revision) = revision_id(revision) {
                    reachable.revisions.insert(revision);
                }
            }
        }
    }
    Ok(reachable)
}

/// Converts a persisted revision identifier to its typed form.
///
/// - `bytes`: Candidate revision identifier bytes.
///
/// A typed revision identity when the input is 32 bytes.
///
/// Returns unit when the input length is invalid.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
fn revision_id(bytes: Vec<u8>) -> Result<RevisionId, ()> {
    if bytes.len() != 32 {
        return Err(());
    }
    let mut value = [0; 32];
    value.copy_from_slice(&bytes);
    Ok(RevisionId::new(value))
}

/// Loads all persistent node identities from canonical storage.
///
/// - `connection`: SQLite connection containing canonical storage.
/// - `crate`: Internal visibility marker.
///
/// Every stored persistent node identity.
///
/// Returns an error when node identities cannot be read.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(crate) fn all_nodes(connection: &Connection) -> rusqlite::Result<BTreeSet<SnapshotNodeId>> {
    let mut result = BTreeSet::new();
    let mut rows = connection.prepare("SELECT id FROM snapshot_nodes")?;
    let ids = rows.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
    for id in ids {
        let id = id?;
        if id.len() == 32 {
            let mut value = [0; 32];
            value.copy_from_slice(&id);
            result.insert(SnapshotNodeId::new(value));
        }
    }
    Ok(result)
}
