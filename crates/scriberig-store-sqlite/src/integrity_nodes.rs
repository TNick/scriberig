//! Validates SQLite snapshot roots and persistent map nodes.

use std::collections::BTreeSet;

use scriberig_core::SnapshotId;
use scriberig_store::{IntegrityIssue, IntegrityReport, SnapshotNodeId};

use crate::snapshot_integrity;

/// Validates all named roots against readable snapshots.
///
/// # Parameters
/// - `connection`: SQLite connection containing named roots.
/// - `valid_snapshots`: Snapshot identities proven valid.
/// - `report`: Integrity report receiving findings.
/// - `crate`: Internal module path used for node access.
///
/// # Returns
/// Unit after all roots are checked.
///
/// # Errors
/// Returns a SQLite query error.
pub(crate) fn validate_roots(
    connection: &rusqlite::Connection,
    valid_snapshots: &BTreeSet<SnapshotId>,
    report: &mut IntegrityReport,
) -> rusqlite::Result<()> {
    let mut rows = connection.prepare("SELECT name,snapshot_id FROM roots")?;
    let roots = rows.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
    })?;
    for root in roots {
        let (name, snapshot) = root?;
        let valid_name = scriberig_core::RootName::new(name.clone()).is_ok();
        let valid_snapshot =
            digest_id(snapshot).is_some_and(|snapshot| valid_snapshots.contains(&snapshot));
        if !valid_name || !valid_snapshot {
            report.corrupt.push(IntegrityIssue::InvalidRoot(name));
        }
    }
    Ok(())
}

/// Verifies every persistent map node and its child references.
///
/// # Parameters
/// - `connection`: SQLite connection containing persistent nodes.
/// - `report`: Integrity report receiving findings.
/// - `crate`: Internal module path used for node enumeration.
///
/// # Returns
/// Unit after all nodes are checked.
///
/// # Errors
/// Returns a SQLite query error when node inspection cannot complete.
pub(crate) fn validate_orphan_nodes(
    connection: &rusqlite::Connection,
    report: &mut IntegrityReport,
) -> rusqlite::Result<()> {
    for node in crate::reachability::all_nodes(connection)? {
        match snapshot_integrity::inspect_nodes(connection, &node.as_bytes()) {
            Ok(_) => {}
            Err(snapshot_integrity::NodeWalkError::Missing(missing)) => {
                if missing.len() == 32 {
                    report
                        .corrupt
                        .push(IntegrityIssue::MissingSnapshotNode(SnapshotNodeId::new(
                            missing.try_into().expect("length checked"),
                        )));
                }
            }
            Err(snapshot_integrity::NodeWalkError::Invalid(_)) => {
                report
                    .corrupt
                    .push(IntegrityIssue::InvalidSnapshotNode(node));
            }
            Err(snapshot_integrity::NodeWalkError::Backend(message)) => {
                return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                    std::io::Error::other(message),
                )));
            }
        }
    }
    Ok(())
}

/// Converts a 32-byte digest into a snapshot identity.
///
/// # Parameters
/// - `bytes`: Candidate digest bytes.
///
/// # Returns
/// A snapshot identity when the input has the required length.
fn digest_id(bytes: Vec<u8>) -> Option<SnapshotId> {
    (bytes.len() == 32).then(|| SnapshotId::new(bytes.try_into().expect("length checked")))
}
