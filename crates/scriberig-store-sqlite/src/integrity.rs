//! Verifies SQLite canonical objects, roots, nodes, and derived indexes.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{OptionalExtension, params};
use scriberig_core::{EncodingVersion, ObjectType, RevisionId, SnapshotId};
use scriberig_store::{
    IndexStatus, IntegrityIssue, IntegrityReport, MaintenanceStore, ProtectedRoots, Revision,
    Snapshot, SnapshotNodeId, UnreachableObjects,
};

use crate::{SqliteStore, integrity_nodes, reachability, snapshot_integrity};

impl MaintenanceStore for SqliteStore {
    /// Verifies SQLite canonical state.
    ///
    /// # Parameters
    /// - `protected`: Stable protected snapshots.
    ///
    /// # Returns
    /// Integrity and reachability findings.
    ///
    /// # Errors
    /// Returns a backend maintenance error.
    fn integrity_check(&self, protected: &ProtectedRoots) -> Result<IntegrityReport, String> {
        Self::integrity_check(self, protected).map_err(|error| error.to_string())
    }

    /// Reports SQLite garbage-collection candidates without deleting them.
    ///
    /// # Parameters
    /// - `protected`: Stable protected snapshots.
    ///
    /// # Returns
    /// A dry-run maintenance report.
    ///
    /// # Errors
    /// Returns a backend maintenance error.
    fn gc_dry_run(&self, protected: &ProtectedRoots) -> Result<IntegrityReport, String> {
        Self::gc_dry_run(self, protected).map_err(|error| error.to_string())
    }

    /// Sweeps SQLite garbage-collection candidates.
    ///
    /// # Parameters
    /// - `protected`: Stable protected snapshots.
    ///
    /// # Returns
    /// The pre-sweep maintenance report.
    ///
    /// # Errors
    /// Returns a backend maintenance error.
    fn gc_sweep(&self, protected: &ProtectedRoots) -> Result<IntegrityReport, String> {
        Self::gc_sweep(self, protected).map_err(|error| error.to_string())
    }

    /// Rebuilds SQLite derived indexes.
    ///
    /// # Returns
    /// A consistent index status.
    ///
    /// # Errors
    /// Returns a backend maintenance error.
    fn rebuild_indexes(&self) -> Result<IndexStatus, String> {
        self.rebuild_derived_indexes()
            .map_err(|error| error.to_string())?;
        Ok(IndexStatus { consistent: true })
    }
}

impl SqliteStore {
    /// Verifies canonical SQLite state and reports valid unreachable objects.
    ///
    /// # Parameters
    /// - `protected`: Stable snapshots that maintenance must retain.
    ///
    /// # Returns
    /// Structured corruption and reachability findings.
    ///
    /// # Errors
    /// Returns a backend error when authoritative rows cannot be inspected.
    pub fn integrity_check(&self, protected: &ProtectedRoots) -> rusqlite::Result<IntegrityReport> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let mut report = IntegrityReport::default();
        let valid_revisions = validate_revisions(&connection, &mut report)?;
        let mut valid_snapshots = validate_snapshots(&connection, &valid_revisions, &mut report)?;
        valid_snapshots.insert(Snapshot::empty().id());
        integrity_nodes::validate_orphan_nodes(&connection, &mut report)?;
        integrity_nodes::validate_roots(&connection, &valid_snapshots, &mut report)?;

        for snapshot_id in &protected.snapshots {
            if *snapshot_id == Snapshot::empty().id() {
                continue;
            }
            if !valid_snapshots.contains(snapshot_id) {
                report
                    .corrupt
                    .push(IntegrityIssue::MissingSnapshot(*snapshot_id));
            }
        }

        let valid_protected: BTreeSet<_> = protected
            .snapshots
            .intersection(&valid_snapshots)
            .copied()
            .collect();
        let marked = reachability::mark(
            &connection,
            &ProtectedRoots {
                snapshots: valid_protected,
            },
        )?;
        let all_snapshots = all_snapshots(&connection)?;
        let all_nodes = reachability::all_nodes(&connection)?;
        report.unreachable = UnreachableObjects {
            snapshots: all_snapshots
                .difference(&marked.snapshots)
                .filter(|id| valid_snapshots.contains(id))
                .copied()
                .collect(),
            revisions: valid_revisions
                .difference(&marked.revisions)
                .copied()
                .collect(),
            snapshot_nodes: all_nodes
                .difference(&marked.snapshot_nodes)
                .copied()
                .collect(),
        };
        Ok(report)
    }
}

/// Validates all persisted revision identities.
///
/// # Parameters
/// - `connection`: SQLite connection containing revisions.
/// - `report`: Integrity report receiving findings.
///
/// # Returns
/// Valid revision identities.
///
/// # Errors
/// Returns a SQLite query error.
fn validate_revisions(
    connection: &rusqlite::Connection,
    report: &mut IntegrityReport,
) -> rusqlite::Result<BTreeSet<RevisionId>> {
    let mut valid = BTreeSet::new();
    let mut rows = connection.prepare("SELECT id,version,object_type,payload FROM revisions")?;
    let revisions = rows.query_map([], |row| {
        Ok((
            row.get::<_, Vec<u8>>(0)?,
            row.get::<_, u16>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Vec<u8>>(3)?,
        ))
    })?;
    for row in revisions {
        let (id, version, object_type, payload) = row?;
        let Some(id) = digest_revision(id) else {
            continue;
        };
        let valid_revision = ObjectType::new(object_type).ok().and_then(|object_type| {
            Revision::new(EncodingVersion::new(version), object_type, payload).ok()
        });
        if valid_revision.is_some_and(|revision| revision.id() == id) {
            valid.insert(id);
        } else {
            report.corrupt.push(IntegrityIssue::RevisionMismatch(id));
        }
    }
    Ok(valid)
}

/// Validates every snapshot-map root and its flat derived index.
///
/// # Parameters
/// - `connection`: SQLite connection containing snapshots.
/// - `valid_revisions`: Revision identities proven canonical.
/// - `report`: Integrity report receiving findings.
///
/// # Returns
/// Valid snapshot identities.
///
/// # Errors
/// Returns a SQLite query error.
fn validate_snapshots(
    connection: &rusqlite::Connection,
    valid_revisions: &BTreeSet<RevisionId>,
    report: &mut IntegrityReport,
) -> rusqlite::Result<BTreeSet<SnapshotId>> {
    let mut valid = BTreeSet::new();
    let mut rows = connection.prepare("SELECT id FROM snapshots")?;
    let snapshots = rows.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
    for row in snapshots {
        let bytes = row?;
        let Some(snapshot_id) = digest_id(bytes) else {
            continue;
        };
        let root: Option<Vec<u8>> = connection
            .query_row(
                "SELECT node_id FROM snapshot_node_roots WHERE snapshot_id=?",
                params![snapshot_id.as_bytes().as_slice()],
                |row| row.get(0),
            )
            .optional()?;
        let entries = if let Some(root) = root {
            match snapshot_integrity::inspect_nodes(connection, &root) {
                Ok(walk) => walk.entries,
                Err(snapshot_integrity::NodeWalkError::Missing(node)) => {
                    if node.len() == 32 {
                        report.corrupt.push(IntegrityIssue::MissingSnapshotNode(
                            SnapshotNodeId::new(node.try_into().expect("length checked")),
                        ));
                    }
                    report
                        .corrupt
                        .push(IntegrityIssue::InvalidSnapshot(snapshot_id));
                    continue;
                }
                Err(snapshot_integrity::NodeWalkError::Invalid(_)) => {
                    if let Some(node) = root.as_slice().try_into().ok().map(SnapshotNodeId::new) {
                        report
                            .corrupt
                            .push(IntegrityIssue::InvalidSnapshotNode(node));
                    }
                    report
                        .corrupt
                        .push(IntegrityIssue::InvalidSnapshot(snapshot_id));
                    continue;
                }
                Err(snapshot_integrity::NodeWalkError::Backend(message)) => {
                    return Err(rusqlite::Error::ToSqlConversionFailure(Box::new(
                        std::io::Error::other(message),
                    )));
                }
            }
        } else {
            load_legacy_entries(connection, snapshot_id)?
        };
        if entries
            .values()
            .any(|revision| !valid_revisions.contains(revision))
        {
            report
                .corrupt
                .push(IntegrityIssue::InvalidSnapshot(snapshot_id));
            continue;
        }
        let rebuilt = Snapshot::from_entries(entries.clone());
        if rebuilt.id() != snapshot_id {
            report
                .corrupt
                .push(IntegrityIssue::InvalidSnapshot(snapshot_id));
            continue;
        }
        if load_index_entries(connection, snapshot_id)? != entries {
            report
                .corrupt
                .push(IntegrityIssue::InvalidIndex(snapshot_id));
        }
        valid.insert(snapshot_id);
    }
    Ok(valid)
}

/// Reads the authoritative snapshot identifiers.
///
/// # Parameters
/// - `connection`: SQLite connection containing snapshots.
///
/// # Returns
/// Snapshot identities stored in the database.
///
/// # Errors
/// Returns a SQLite query error.
fn all_snapshots(connection: &rusqlite::Connection) -> rusqlite::Result<BTreeSet<SnapshotId>> {
    let mut result = BTreeSet::new();
    let mut rows = connection.prepare("SELECT id FROM snapshots")?;
    let snapshots = rows.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
    for snapshot in snapshots.flatten() {
        if let Some(snapshot) = digest_id(snapshot) {
            result.insert(snapshot);
        }
    }
    Ok(result)
}

/// Loads an old-format snapshot while preserving compatibility with schema 1.
///
/// # Parameters
/// - `connection`: SQLite connection containing legacy entries.
/// - `snapshot`: Snapshot whose entries are loaded.
///
/// # Returns
/// The legacy entity-to-revision mapping.
///
/// # Errors
/// Returns a SQLite query or identifier-shape error.
fn load_legacy_entries(
    connection: &rusqlite::Connection,
    snapshot: SnapshotId,
) -> rusqlite::Result<BTreeMap<scriberig_core::EntityId, RevisionId>> {
    let mut result = BTreeMap::new();
    let mut rows =
        connection.prepare("SELECT entity,revision FROM snapshot_entries WHERE snapshot_id=?")?;
    let entries = rows.query_map(params![snapshot.as_bytes().as_slice()], |row| {
        Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?))
    })?;
    for entry in entries {
        let (entity, revision) = entry?;
        if entity.len() != 16 || revision.len() != 32 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let mut entity_id = [0; 16];
        entity_id.copy_from_slice(&entity);
        let mut revision_id = [0; 32];
        revision_id.copy_from_slice(&revision);
        result.insert(
            scriberig_core::EntityId::new(entity_id),
            RevisionId::new(revision_id),
        );
    }
    Ok(result)
}

/// Loads the rebuildable flat snapshot-entry index.
///
/// # Parameters
/// - `connection`: SQLite connection containing index rows.
/// - `snapshot`: Snapshot whose index is loaded.
///
/// # Returns
/// The indexed entity-to-revision mapping.
///
/// # Errors
/// Returns a SQLite query or identifier-shape error.
fn load_index_entries(
    connection: &rusqlite::Connection,
    snapshot: SnapshotId,
) -> rusqlite::Result<BTreeMap<scriberig_core::EntityId, RevisionId>> {
    load_legacy_entries(connection, snapshot)
}

/// Converts a 32-byte digest into a snapshot or revision identity.
///
/// # Parameters
/// - `bytes`: Candidate digest bytes.
///
/// # Returns
/// A snapshot identity when the input has the required length.
fn digest_id(bytes: Vec<u8>) -> Option<SnapshotId> {
    (bytes.len() == 32).then(|| SnapshotId::new(bytes.try_into().expect("length checked")))
}

/// Converts a 32-byte digest into a revision identity.
///
/// # Parameters
/// - `bytes`: Candidate digest bytes.
///
/// # Returns
/// A revision identity when the input has the required length.
fn digest_revision(bytes: Vec<u8>) -> Option<RevisionId> {
    (bytes.len() == 32).then(|| RevisionId::new(bytes.try_into().expect("length checked")))
}
