//! Exercises SQLite maintenance, reachability, and canonical integrity checks.

use std::fs;
use std::path::PathBuf;

use scriberig_core::{EncodingVersion, EntityId, ObjectType};
use scriberig_store::{
    ProtectedRoots, Revision, RevisionStore, SnapshotChange, SnapshotStore, StoreObservability,
};
use scriberig_store_sqlite::SqliteStore;

/// Returns an isolated temporary SQLite database path.
///
/// # Parameters
/// - `label`: Test-specific filename label.
///
/// # Returns
/// A path in the platform temporary directory.
fn temporary_database_path(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("scriberig-{label}-{}.db", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}

/// Verifies reachability preserves the union of two protected branches.
#[test]
fn two_protected_roots_preserve_their_union() {
    let store = SqliteStore::open_in_memory().expect("open");
    let kind = ObjectType::new("test.union").expect("type");
    let first = store
        .put(Revision::new(EncodingVersion::V1, kind.clone(), vec![1]).expect("revision"))
        .expect("first");
    let second = store
        .put(Revision::new(EncodingVersion::V1, kind, vec![2]).expect("revision"))
        .expect("second");
    let base = scriberig_store::Snapshot::empty().id();
    let left = store
        .apply_changes(base, &[SnapshotChange::Set(EntityId::new([1; 16]), first)])
        .expect("left");
    let right = store
        .apply_changes(base, &[SnapshotChange::Set(EntityId::new([2; 16]), second)])
        .expect("right");
    let report = store
        .gc_dry_run(&ProtectedRoots {
            snapshots: [left, right].into_iter().collect(),
        })
        .expect("dry run");
    assert_eq!(report.corrupt, []);
    assert!(!report.unreachable.snapshots.contains(&left));
    assert!(!report.unreachable.snapshots.contains(&right));
    assert!(!report.unreachable.revisions.contains(&first));
    assert!(!report.unreachable.revisions.contains(&second));
}

/// Verifies deleting a persistent map node is canonical corruption.
#[test]
fn missing_snapshot_node_is_corruption_and_not_swept() {
    let path = temporary_database_path("missing-node");
    let (snapshot, entity) = {
        let store = SqliteStore::open(&path).expect("open");
        let revision = store
            .put(
                Revision::new(
                    EncodingVersion::V1,
                    ObjectType::new("test.node").expect("type"),
                    vec![3],
                )
                .expect("revision"),
            )
            .expect("put");
        let entity = EntityId::new([6; 16]);
        let snapshot = store
            .apply_changes(
                scriberig_store::Snapshot::empty().id(),
                &[SnapshotChange::Set(entity, revision)],
            )
            .expect("snapshot");
        (snapshot, entity)
    };
    let connection = rusqlite::Connection::open(&path).expect("connection");
    let node: Vec<u8> = connection
        .query_row(
            "SELECT node_id FROM snapshot_node_roots WHERE snapshot_id=?",
            rusqlite::params![snapshot.as_bytes().as_slice()],
            |row| row.get(0),
        )
        .expect("node root");
    connection
        .execute(
            "DELETE FROM snapshot_nodes WHERE id=?",
            rusqlite::params![node],
        )
        .expect("delete node");
    drop(connection);
    let store = SqliteStore::open(&path).expect("reopen");
    let report = store
        .integrity_check(&ProtectedRoots {
            snapshots: [snapshot].into_iter().collect(),
        })
        .expect("integrity");
    assert!(report.corrupt.iter().any(|issue| matches!(
        issue,
        scriberig_store::IntegrityIssue::MissingSnapshotNode(_)
    )));
    assert!(store.resolve(snapshot, entity).is_err());
    let sweep = store
        .gc_sweep(&ProtectedRoots {
            snapshots: [snapshot].into_iter().collect(),
        })
        .expect("safe sweep");
    assert_ne!(sweep.corrupt, []);
    assert_eq!(store.stats().expect("stats").snapshots, Some(1));
    drop(store);
    fs::remove_file(path).expect("cleanup");
}

/// Verifies a deleted derived index is rebuilt without changing snapshot identity.
#[test]
fn derived_snapshot_index_rebuild_preserves_identity() {
    let path = temporary_database_path("index");
    let (snapshot, entity, revision) = {
        let store = SqliteStore::open(&path).expect("open");
        let revision = store
            .put(
                Revision::new(
                    EncodingVersion::V1,
                    ObjectType::new("test.index").expect("type"),
                    vec![4],
                )
                .expect("revision"),
            )
            .expect("put");
        let entity = EntityId::new([7; 16]);
        let snapshot = store
            .apply_changes(
                scriberig_store::Snapshot::empty().id(),
                &[SnapshotChange::Set(entity, revision)],
            )
            .expect("snapshot");
        (snapshot, entity, revision)
    };
    let connection = rusqlite::Connection::open(&path).expect("connection");
    connection
        .execute("DELETE FROM snapshot_entries", [])
        .expect("delete derived index");
    drop(connection);
    let store = SqliteStore::open(&path).expect("reopen");
    let report = store
        .integrity_check(&ProtectedRoots {
            snapshots: [snapshot].into_iter().collect(),
        })
        .expect("integrity");
    assert!(report.corrupt.iter().any(|issue| matches!(
        issue,
        scriberig_store::IntegrityIssue::InvalidIndex(found) if *found == snapshot
    )));
    store.rebuild_derived_indexes().expect("rebuild");
    let repaired = store
        .integrity_check(&ProtectedRoots {
            snapshots: [snapshot].into_iter().collect(),
        })
        .expect("recheck");
    assert_eq!(repaired.corrupt, []);
    assert_eq!(store.resolve(snapshot, entity), Ok(Some(revision)));
    drop(store);
    fs::remove_file(path).expect("cleanup");
}
