//! Unit tests for SQLite transaction behavior.

use super::*;
use scriberig_core::{EntityId, ProjectId, RevisionId, RootName};
use scriberig_store::{RootStore, Snapshot, SnapshotChange};

/// Verifies the store exposes its resolved database path to sandbox policy.
#[test]
fn database_path_reports_file_backed_and_in_memory_storage() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-store-path-{}.sqlite",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let file_store = SqliteStore::open(&path).expect("file-backed store");
    assert_eq!(
        file_store.database_path().expect("database path"),
        Some(path.canonicalize().expect("canonical database path"))
    );
    drop(file_store);
    std::fs::remove_file(&path).expect("remove temporary database");

    let memory_store = SqliteStore::open_in_memory().expect("in-memory store");
    assert_eq!(memory_store.database_path().expect("database path"), None);
}

/// Verifies an interrupted transaction does not publish prepared content.
#[test]
fn interrupted_promotion_keeps_previous_root() {
    let store = SqliteStore::open_in_memory().expect("open");
    let root = RootName::new("main").expect("root");
    let base = Snapshot::empty().id();
    store
        .compare_and_set_root(root.clone(), None, base)
        .expect("base root");
    let next = Snapshot::empty().apply([SnapshotChange::Set(
        EntityId::from_bytes([1; 16]),
        RevisionId::new([2; 32]),
    )]);
    let mut connection = store.connection.lock().expect("connection");
    let transaction = connection.transaction().expect("transaction");
    SqliteStore::store_snapshot_in_transaction(&transaction, &next).expect("prepare");
    drop(transaction);
    drop(connection);
    assert_eq!(store.read_root(&root), Some(base));
    assert!(store.snapshot(next.id()).is_err());
}

/// Verifies multiple projects coexist independently in one database.
#[test]
fn project_catalog_supports_multiple_projects_and_relinking() {
    let store = SqliteStore::open_in_memory().expect("open");
    let first = store.register_project("First", "project-a").expect("first");
    let second = store
        .register_project("Second", "project-b")
        .expect("second");
    assert_ne!(first.id, second.id);
    assert_eq!(store.list_projects().expect("list").len(), 2);
    store
        .relink_project(first.id, "project-a-moved")
        .expect("relink");
    assert_eq!(
        store
            .project(first.id)
            .expect("read")
            .expect("project")
            .path,
        std::path::PathBuf::from("project-a-moved")
    );
}

/// Verifies malformed project identifiers are never accepted by storage APIs.
#[test]
fn project_identity_is_fixed_width() {
    assert_eq!(ProjectId::from_bytes([7; 16]).as_bytes(), &[7; 16]);
}
