//! Verifies atomic SQLite promotion and metadata recovery behavior.

use std::fs;
use std::path::PathBuf;

use scriberig_core::{EncodingVersion, EntityId, ObjectType, RevisionId, RootName};
use scriberig_store::{Revision, RevisionStore, RootStore, SnapshotChange, StoreObservability};
use scriberig_store_sqlite::SqliteStore;

/// Verifies revisions and promotion commit as one durable transaction.
#[test]
fn atomic_revision_insert_and_promotion_commits_together() {
    let store = SqliteStore::open_in_memory().expect("open");
    let root = RootName::new("main").expect("root");
    let base = scriberig_store::Snapshot::empty().id();
    let revision = Revision::new(
        EncodingVersion::V1,
        ObjectType::new("test.atomic").expect("type"),
        vec![8],
    )
    .expect("revision");
    let entity = EntityId::from_bytes([4; 16]);
    let snapshot = store
        .apply_revisions_and_promote(
            root.clone(),
            None,
            base,
            std::slice::from_ref(&revision),
            &[SnapshotChange::Set(entity, revision.id())],
        )
        .expect("atomic promotion");

    assert_eq!(store.read_root(&root), Some(snapshot));
    assert_eq!(store.get(revision.id()).expect("revision"), Some(revision));
}

/// Verifies snapshots cannot publish references to missing revisions.
#[test]
fn missing_revision_rejects_promotion_and_rolls_back() {
    let store = SqliteStore::open_in_memory().expect("open");
    let root = RootName::new("main").expect("root");
    let missing = RevisionId::new([77; 32]);
    let entity = EntityId::from_bytes([5; 16]);
    let error = store
        .apply_changes_and_promote(
            root.clone(),
            None,
            scriberig_store::Snapshot::empty().id(),
            &[SnapshotChange::Set(entity, missing)],
        )
        .expect_err("missing revision");

    assert_eq!(
        error,
        scriberig_store_sqlite::PromotionError::MissingRevision(missing)
    );
    assert_eq!(store.read_root(&root), None);
    assert_eq!(store.stats().expect("stats").snapshots, Some(0));
}

/// Verifies an unsupported newer backend schema is rejected explicitly.
#[test]
fn newer_schema_version_is_rejected() {
    let path = temporary_database_path("schema");
    {
        let store = SqliteStore::open(&path).expect("open");
        assert_eq!(store.schema_version().expect("version"), 5);
    }
    let connection = rusqlite::Connection::open(&path).expect("connection");
    connection
        .execute(
            "UPDATE metadata SET value='99' WHERE key='schema_version'",
            [],
        )
        .expect("update version");
    drop(connection);
    let Err(error) = SqliteStore::open(&path) else {
        panic!("newer schema was accepted")
    };
    assert!(
        error
            .to_string()
            .contains("unsupported SQLite backend schema version")
    );
    fs::remove_file(path).expect("cleanup");
}

/// Verifies incompatible recovery metadata is rejected during initialization.
#[test]
fn incompatible_recovery_metadata_is_rejected() {
    let path = temporary_database_path("metadata");
    {
        let store = SqliteStore::open(&path).expect("open");
        assert_eq!(store.schema_version().expect("version"), 5);
    }
    let connection = rusqlite::Connection::open(&path).expect("connection");
    connection
        .execute(
            "UPDATE metadata SET value='other' WHERE key='recovery_mode'",
            [],
        )
        .expect("update mode");
    drop(connection);

    let error = SqliteStore::open(&path).expect_err("incompatible metadata");
    assert!(error.to_string().contains("unsupported SQLite metadata"));
    fs::remove_file(path).expect("cleanup");
}

/// Returns a unique temporary database path for one durable test.
///
/// # Parameters
/// - `label`: Test-specific path label.
///
/// # Returns
/// A unique temporary database path.
fn temporary_database_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "scriberig-sqlite-{label}-{}-{}.db",
        std::process::id(),
        unique_test_suffix()
    ))
}

/// Produces a process-local suffix without relying on wall-clock ordering.
///
/// # Returns
/// A high-resolution process-local suffix.
fn unique_test_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos()
}
