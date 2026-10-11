//! Validates and traverses SQLite persistent snapshot-map nodes.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{Connection, params};
use scriberig_core::{EntityId, RevisionId};
use scriberig_store::SnapshotNodeId;

/// Computes a content identity for one persistent SQLite map node.
///
/// # Parameters
/// - `kind`: Node kind discriminator.
/// - `entity`: Optional leaf entity bytes.
/// - `revision`: Optional leaf revision bytes.
/// - `left`: Optional left child identity.
/// - `right`: Optional right child identity.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// The content-derived node identity bytes.
pub(crate) fn node_id(
    kind: u8,
    entity: Option<&[u8]>,
    revision: Option<&[u8]>,
    left: Option<&[u8]>,
    right: Option<&[u8]>,
) -> Vec<u8> {
    let mut bytes = vec![kind];
    for value in [entity, revision, left, right] {
        bytes.extend_from_slice(value.unwrap_or(&[]));
        bytes.push(0xff);
    }
    blake3::hash(&bytes).as_bytes().to_vec()
}

/// Reports a persistent snapshot-map node inspection failure.
#[derive(Debug)]
pub(crate) enum NodeWalkError {
    /// A referenced node row is absent.
    Missing(Vec<u8>),
    /// A node row violates the canonical node structure.
    Invalid(&'static str),
    /// SQLite returned an operational error.
    Backend(String),
}

/// Contains the authoritative objects found below one snapshot root.
#[derive(Debug, Default)]
pub(crate) struct NodeWalk {
    /// Persistent node identities encountered during traversal.
    pub nodes: BTreeSet<SnapshotNodeId>,
    /// Entity-to-revision mappings found in the leaves.
    pub entries: BTreeMap<EntityId, RevisionId>,
}

/// Loads and validates a persistent node tree into canonical entries.
///
/// # Parameters
/// - `connection`: SQLite connection containing the node records.
/// - `id`: Root node identity.
/// - `entries`: Destination mapping for loaded entries.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Unit after the node tree is loaded.
///
/// # Errors
/// Returns a SQLite or structural-integrity error.
pub(crate) fn load_node(
    connection: &Connection,
    id: &[u8],
    entries: &mut BTreeMap<EntityId, RevisionId>,
) -> rusqlite::Result<()> {
    let walk = inspect_nodes(connection, id).map_err(|error| match error {
        NodeWalkError::Backend(message) => {
            rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::other(message)))
        }
        NodeWalkError::Missing(node) => rusqlite::Error::ToSqlConversionFailure(Box::new(
            std::io::Error::other(format!("missing snapshot node: {node:?}")),
        )),
        NodeWalkError::Invalid(reason) => rusqlite::Error::ToSqlConversionFailure(Box::new(
            std::io::Error::other(format!("invalid snapshot node: {reason}")),
        )),
    })?;
    entries.extend(walk.entries);
    Ok(())
}

/// Inspects and validates a persistent snapshot-map node graph.
///
/// # Parameters
/// - `connection`: SQLite connection containing node records.
/// - `root`: Root node identity.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// All validated nodes and leaf mappings below the root.
///
/// # Errors
/// Returns a missing-node, structural-integrity, or SQLite error.
pub(crate) fn inspect_nodes(
    connection: &Connection,
    root: &[u8],
) -> Result<NodeWalk, NodeWalkError> {
    let mut walk = NodeWalk::default();
    let mut visiting = BTreeSet::new();
    inspect_node(connection, root, &mut visiting, &mut walk)?;
    Ok(walk)
}

/// Recursively validates one node and its children.
///
/// # Parameters
/// - `connection`: SQLite connection containing node records.
/// - `id`: Node identity to inspect.
/// - `visiting`: Active recursion path used for cycle detection.
/// - `walk`: Accumulator for validated nodes and entries.
/// - `crate`: Internal visibility marker.
///
/// # Returns
/// Number of entries below the node.
///
/// # Errors
/// Returns a missing-node, structural-integrity, or SQLite error.
fn inspect_node(
    connection: &Connection,
    id: &[u8],
    visiting: &mut BTreeSet<Vec<u8>>,
    walk: &mut NodeWalk,
) -> Result<usize, NodeWalkError> {
    if id.len() != 32 {
        return Err(NodeWalkError::Invalid("node identity length"));
    }
    if !visiting.insert(id.to_vec()) {
        return Err(NodeWalkError::Invalid("node cycle"));
    }
    let row = connection
        .query_row(
            "SELECT kind,entity,revision,left_child,right_child,item_count
             FROM snapshot_nodes WHERE id=?",
            params![id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<Vec<u8>>>(1)?,
                    row.get::<_, Option<Vec<u8>>>(2)?,
                    row.get::<_, Option<Vec<u8>>>(3)?,
                    row.get::<_, Option<Vec<u8>>>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .map_err(|error| {
            if matches!(error, rusqlite::Error::QueryReturnedNoRows) {
                NodeWalkError::Missing(id.to_vec())
            } else {
                NodeWalkError::Backend(error.to_string())
            }
        })?;
    let expected_id = node_id(
        u8::try_from(row.0).map_err(|_| NodeWalkError::Invalid("node kind"))?,
        row.1.as_deref(),
        row.2.as_deref(),
        row.3.as_deref(),
        row.4.as_deref(),
    );
    if expected_id != id {
        return Err(NodeWalkError::Invalid("node identity"));
    }
    walk.nodes.insert(SnapshotNodeId::new(
        id.try_into().expect("validated node identity"),
    ));
    let count = match row.0 {
        0 if row.1.is_none()
            && row.2.is_none()
            && row.3.is_none()
            && row.4.is_none()
            && row.5 == 0 =>
        {
            0
        }
        1 if row.1.as_ref().is_some_and(|value| value.len() == 16)
            && row.2.as_ref().is_some_and(|value| value.len() == 32)
            && row.3.is_none()
            && row.4.is_none()
            && row.5 == 1 =>
        {
            let mut entity = [0; 16];
            entity.copy_from_slice(row.1.as_ref().expect("checked leaf entity"));
            let mut revision = [0; 32];
            revision.copy_from_slice(row.2.as_ref().expect("checked leaf revision"));
            if walk
                .entries
                .insert(EntityId::new(entity), RevisionId::new(revision))
                .is_some()
            {
                return Err(NodeWalkError::Invalid("duplicate entity leaf"));
            }
            1
        }
        2 if row.1.is_none()
            && row.2.is_none()
            && row.3.as_ref().is_some_and(|value| value.len() == 32)
            && row.4.as_ref().is_some_and(|value| value.len() == 32) =>
        {
            let left = inspect_node(
                connection,
                row.3.as_ref().expect("checked left child"),
                visiting,
                walk,
            )?;
            let right = inspect_node(
                connection,
                row.4.as_ref().expect("checked right child"),
                visiting,
                walk,
            )?;
            if i64::try_from(left + right).ok() != Some(row.5) {
                return Err(NodeWalkError::Invalid("branch item count"));
            }
            left + right
        }
        _ => return Err(NodeWalkError::Invalid("node fields")),
    };
    visiting.remove(id);
    Ok(count)
}
