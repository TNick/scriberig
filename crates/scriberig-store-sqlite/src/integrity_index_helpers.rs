//! Contains SQLite integrity index helper functions.

use super::{BTreeMap, RevisionId, SnapshotId, load_legacy_entries};

/// Loads the rebuildable flat snapshot-entry index.
///
/// - `connection`: SQLite connection containing index rows.
/// - `snapshot`: Snapshot whose index is loaded.
///
/// The indexed entity-to-revision mapping.
///
/// Returns a SQLite query or identifier-shape error.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub(super) fn load_index_entries(
    connection: &rusqlite::Connection,
    snapshot: SnapshotId,
) -> rusqlite::Result<BTreeMap<scriberig_core::EntityId, RevisionId>> {
    load_legacy_entries(connection, snapshot)
}

/// Converts a 32-byte digest into a snapshot or revision identity.
///
/// - `bytes`: Candidate digest bytes.
///
/// A snapshot identity when the input has the required length.
/// # Returns
/// The operation result.
pub(super) fn digest_id(bytes: Vec<u8>) -> Option<SnapshotId> {
    (bytes.len() == 32).then(|| SnapshotId::new(bytes.try_into().expect("length checked")))
}

/// Converts a 32-byte digest into a revision identity.
///
/// - `bytes`: Candidate digest bytes.
///
/// A revision identity when the input has the required length.
/// # Returns
/// The operation result.
pub(super) fn digest_revision(bytes: Vec<u8>) -> Option<RevisionId> {
    (bytes.len() == 32).then(|| RevisionId::new(bytes.try_into().expect("length checked")))
}
