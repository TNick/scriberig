//! Reads raw durable rows so plan write and read scope can be measured directly.

use std::collections::BTreeMap;
use std::path::Path;

use rusqlite::{Connection, params};
use scriberig_core::{EntityId, SnapshotId};

/// One immutable universal-store revision row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RevisionRow {
    /// Stable semantic object type name.
    pub(super) object_type: String,
    /// Immutable canonical payload bytes.
    pub(super) payload: Vec<u8>,
}

/// One derived `plan_node_rollup_index` row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RollupRow {
    /// Structural parent recorded for the node.
    pub(super) parent_node_id: Option<String>,
    /// Canonical node revision the cached row describes.
    pub(super) node_revision: String,
    /// Node status stored by the canonical plan object.
    pub(super) direct_status: String,
    /// Derived subtree status.
    pub(super) effective_status: String,
    /// Canonical snapshot the cache row is valid for.
    pub(super) source_snapshot: String,
    /// Encoded derived rollup payload.
    pub(super) payload: Vec<u8>,
}

/// Reads every durable revision row keyed by its immutable revision ID.
///
/// # Parameters
/// - `path`: Database shared by the session and universal stores.
///
/// # Returns
/// Durable revision rows keyed by revision ID bytes.
pub(super) fn revision_rows(path: &Path) -> BTreeMap<Vec<u8>, RevisionRow> {
    let connection = Connection::open(path).expect("open durable revision reader");
    let mut statement = connection
        .prepare("SELECT id, object_type, payload FROM revisions")
        .expect("prepare durable revision query");
    statement
        .query_map([], |row| {
            Ok((
                row.get::<_, Vec<u8>>(0)?,
                RevisionRow {
                    object_type: row.get(1)?,
                    payload: row.get(2)?,
                },
            ))
        })
        .expect("read durable revision rows")
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .expect("collect durable revision rows")
}

/// Reads the entity-to-revision mapping stored in one exact snapshot.
///
/// # Parameters
/// - `path`: Database shared by the session and universal stores.
/// - `snapshot`: Exact universal snapshot whose entries are read.
///
/// # Returns
/// Snapshot entries keyed by entity bytes and valued by revision bytes.
pub(super) fn snapshot_entries(path: &Path, snapshot: &SnapshotId) -> BTreeMap<Vec<u8>, Vec<u8>> {
    let connection = Connection::open(path).expect("open durable snapshot reader");
    let mut statement = connection
        .prepare("SELECT entity, revision FROM snapshot_entries WHERE snapshot_id=?1")
        .expect("prepare durable snapshot entry query");
    statement
        .query_map(params![snapshot.as_bytes().as_slice()], |row| {
            Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?))
        })
        .expect("read durable snapshot entries")
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .expect("collect durable snapshot entries")
}

/// Counts stored semantic objects by type inside one exact snapshot.
///
/// # Parameters
/// - `path`: Database shared by the session and universal stores.
/// - `snapshot`: Exact universal snapshot whose objects are counted.
///
/// # Returns
/// Object-type name to stored row count for the snapshot.
pub(super) fn snapshot_object_counts(path: &Path, snapshot: &SnapshotId) -> BTreeMap<String, i64> {
    let connection = Connection::open(path).expect("open durable snapshot reader");
    let mut statement = connection
        .prepare(
            "SELECT revisions.object_type, COUNT(*)
             FROM snapshot_entries AS entries
             JOIN revisions ON revisions.id=entries.revision
             WHERE entries.snapshot_id=?1
             GROUP BY revisions.object_type",
        )
        .expect("prepare durable object count query");
    statement
        .query_map(params![snapshot.as_bytes().as_slice()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .expect("count durable snapshot objects")
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .expect("collect durable object counts")
}

/// Reads the raw derived rollup rows for one session plan.
///
/// # Parameters
/// - `path`: Database holding the session projection tables.
/// - `session`: Owning durable session identity.
/// - `plan`: Stable semantic plan identity.
///
/// # Returns
/// Rollup rows keyed by stored node identity text.
pub(super) fn rollup_rows(
    path: &Path,
    session: EntityId,
    plan: EntityId,
) -> BTreeMap<String, RollupRow> {
    let connection = Connection::open(path).expect("open durable rollup reader");
    let mut statement = connection
        .prepare(
            "SELECT node_id, parent_node_id, node_revision, direct_status,
                    effective_status, source_snapshot, payload
             FROM plan_node_rollup_index WHERE session_id=?1 AND plan_id=?2",
        )
        .expect("prepare durable rollup query");
    statement
        .query_map(params![session.to_string(), plan.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                RollupRow {
                    parent_node_id: row.get(1)?,
                    node_revision: row.get(2)?,
                    direct_status: row.get(3)?,
                    effective_status: row.get(4)?,
                    source_snapshot: row.get(5)?,
                    payload: row.get(6)?,
                },
            ))
        })
        .expect("read durable rollup rows")
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .expect("collect durable rollup rows")
}

/// Lists snapshot entities whose stored payload embeds one entity identity.
///
/// # Parameters
/// - `path`: Database shared by the session and universal stores.
/// - `snapshot`: Exact universal snapshot whose payloads are scanned.
/// - `identity`: Entity identity searched inside the stored payloads.
///
/// # Returns
/// Entity bytes of every object whose payload contains `identity`.
pub(super) fn entities_holding_identity(
    path: &Path,
    snapshot: &SnapshotId,
    identity: EntityId,
) -> Vec<Vec<u8>> {
    let connection = Connection::open(path).expect("open durable payload reader");
    let mut statement = connection
        .prepare(
            "SELECT entries.entity, revisions.payload
             FROM snapshot_entries AS entries
             JOIN revisions ON revisions.id=entries.revision
             WHERE entries.snapshot_id=?1",
        )
        .expect("prepare durable payload scan");
    let target = serde_json::to_value(identity).expect("encode identity");
    statement
        .query_map(params![snapshot.as_bytes().as_slice()], |row| {
            let entity = row.get::<_, Vec<u8>>(0)?;
            let payload = row.get::<_, Vec<u8>>(1)?;
            let holds = serde_json::from_slice::<serde_json::Value>(&payload)
                .is_ok_and(|value| holds_value(&value, &target));
            Ok((entity, holds))
        })
        .expect("scan durable payloads")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect durable payload scan")
        .into_iter()
        .filter(|(_, holds)| *holds)
        .map(|(entity, _)| entity)
        .collect()
}

/// Recursively searches a decoded semantic payload for one encoded identity.
///
/// # Parameters
/// - `value`: Decoded payload value to search.
/// - `target`: Encoded identity to match exactly.
///
/// # Returns
/// Whether any nested value equals `target`.
fn holds_value(value: &serde_json::Value, target: &serde_json::Value) -> bool {
    value == target
        || match value {
            serde_json::Value::Object(entries) => {
                entries.values().any(|item| holds_value(item, target))
            }
            serde_json::Value::Array(items) => items.iter().any(|item| holds_value(item, target)),
            _ => false,
        }
}
