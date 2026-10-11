//! Exercises durable SQLite storage contracts.
use scriberig_core::{EncodingVersion, EntityId, ObjectType, RevisionId, RootName};
use scriberig_store::{
    ProtectedRoots, Revision, RevisionStore, RootStore, SnapshotChange, SnapshotStore,
    StoreObservability,
};
use scriberig_store_sqlite::SqliteStore;
use std::fs;
use std::path::PathBuf;

/// Verifies revisions, snapshots, and root CAS work in SQLite.
#[test]
fn sqlite_round_trip_and_stale_cas() {
    let store = SqliteStore::open_in_memory().expect("open");
    let kind = ObjectType::new("test").expect("type");
    let revision = Revision::new(EncodingVersion::new(1), kind, vec![1, 2]).expect("revision");
    let revision_id = store.put(revision.clone()).expect("put");
    assert_eq!(store.get(revision_id).expect("get"), Some(revision));
    let base = scriberig_store::Snapshot::empty().id();
    let entity = EntityId::new([1; 16]);
    let snapshot = store
        .apply_changes(base, &[SnapshotChange::Set(entity, revision_id)])
        .expect("snapshot");
    assert_eq!(store.resolve(snapshot, entity), Ok(Some(revision_id)));
    let root = RootName::new("main").expect("root");
    store
        .compare_and_set_root(root.clone(), None, base)
        .expect("root");
    assert!(
        store
            .compare_and_set_root(root.clone(), None, snapshot)
            .is_err()
    );
    assert_eq!(store.read_root(&root), Some(base));
}

/// Verifies inserting one immutable revision twice preserves one logical row.
#[test]
fn duplicate_revision_insert_is_idempotent() {
    let store = SqliteStore::open_in_memory().expect("open");
    let kind = ObjectType::new("test.duplicate").expect("type");
    let revision = Revision::new(EncodingVersion::V1, kind, vec![1, 2]).expect("revision");

    let first = store.put(revision.clone()).expect("first put");
    let second = store.put(revision).expect("second put");

    assert_eq!(first, second);
    assert_eq!(store.stats().expect("stats").revisions, Some(1));
}

/// Verifies an absent root can be read after fresh initialization.
#[test]
fn sqlite_starts_without_roots() {
    let store = SqliteStore::open_in_memory().expect("open");
    let root = RootName::new("main").expect("root");
    assert_eq!(store.read_root(&root), None);
    assert!(!store.contains(RevisionId::new([4; 32])).expect("contains"));
}

/// Verifies dry-run and sweep collection preserve protected snapshots.
#[test]
fn sqlite_gc_preserves_protected_and_collects_abandoned_branch() {
    let store = SqliteStore::open_in_memory().expect("open");
    let kind = ObjectType::new("test").expect("type");
    let first = store
        .put(Revision::new(EncodingVersion::new(1), kind.clone(), vec![1]).expect("revision"))
        .expect("first revision");
    let second = store
        .put(Revision::new(EncodingVersion::new(1), kind, vec![2]).expect("revision"))
        .expect("second revision");
    let entity = EntityId::new([2; 16]);
    let protected = store
        .apply_changes(
            scriberig_store::Snapshot::empty().id(),
            &[SnapshotChange::Set(entity, first)],
        )
        .expect("protected snapshot");
    let abandoned = store
        .apply_changes(
            scriberig_store::Snapshot::empty().id(),
            &[SnapshotChange::Set(entity, second)],
        )
        .expect("abandoned snapshot");
    let roots = ProtectedRoots {
        snapshots: [protected].into_iter().collect(),
    };
    let dry_run = store.gc_dry_run(&roots).expect("dry run");
    assert!(dry_run.unreachable.snapshots.contains(&abandoned));
    assert!(dry_run.unreachable.revisions.contains(&second));
    assert!(!dry_run.unreachable.snapshot_nodes.is_empty());
    assert_eq!(dry_run.corrupt, []);
    assert!(store.resolve(abandoned, entity).is_ok());
    let report = store.gc_sweep(&roots).expect("sweep");
    assert!(report.unreachable.snapshots.contains(&abandoned));
    assert_eq!(store.resolve(protected, entity), Ok(Some(first)));
    assert!(store.resolve(abandoned, entity).is_err());
    store.rebuild_derived_indexes().expect("rebuild indexes");
}

/// Verifies valid unreachable revisions are separate from corruption.
#[test]
fn unreachable_valid_revision_is_not_corrupt() {
    let store = SqliteStore::open_in_memory().expect("open");
    let revision = store
        .put(
            Revision::new(
                EncodingVersion::V1,
                ObjectType::new("test.valid").expect("type"),
                vec![5],
            )
            .expect("revision"),
        )
        .expect("put");
    let report = store
        .integrity_check(&ProtectedRoots::default())
        .expect("integrity");
    assert_eq!(report.corrupt, []);
    assert!(report.unreachable.revisions.contains(&revision));
}

/// Verifies a protected snapshot referencing a missing revision is corrupt.
#[test]
fn missing_referenced_revision_is_corruption() {
    let path = temporary_database_path("missing-revision");
    let store = SqliteStore::open(&path).expect("open");
    let revision = store
        .put(
            Revision::new(
                EncodingVersion::V1,
                ObjectType::new("test.reference").expect("type"),
                vec![6],
            )
            .expect("revision"),
        )
        .expect("put");
    let snapshot = store
        .apply_changes(
            scriberig_store::Snapshot::empty().id(),
            &[SnapshotChange::Set(EntityId::new([9; 16]), revision)],
        )
        .expect("snapshot");
    drop(store);
    let connection = rusqlite::Connection::open(&path).expect("connection");
    connection
        .execute(
            "DELETE FROM revisions WHERE id=?",
            rusqlite::params![revision.as_bytes().as_slice()],
        )
        .expect("delete revision");
    drop(connection);
    let store = SqliteStore::open(&path).expect("reopen");
    let report = store
        .integrity_check(&ProtectedRoots {
            snapshots: [snapshot].into_iter().collect(),
        })
        .expect("integrity");
    assert!(
        report
            .corrupt
            .contains(&scriberig_store::IntegrityIssue::InvalidSnapshot(snapshot))
    );
    drop(store);
    fs::remove_file(path).expect("cleanup");
}

/// Runs the shared backend-neutral conformance suite against SQLite.
#[test]
fn sqlite_shared_conformance_passes() {
    let store = SqliteStore::open_in_memory().expect("open");
    scriberig_store::run_core_conformance(&store).expect("conformance");
}

/// Verifies SQLite observability reports counts without changing state.
#[test]
fn sqlite_observability_reports_counts() {
    let store = SqliteStore::open_in_memory().expect("open");
    let stats = store.stats().expect("stats");
    assert_eq!(stats.revisions, Some(0));
    assert_eq!(stats.snapshots, Some(0));
}

/// Verifies equivalent canonical mappings have backend-independent identities.
#[test]
fn logical_ids_match_memory_backend() {
    use scriberig_store_memory::MemoryStore;
    let sqlite = SqliteStore::open_in_memory().expect("open sqlite");
    let memory = MemoryStore::new();
    let kind = ObjectType::new("conformance.item").expect("type");
    let revision = Revision::new(EncodingVersion::V1, kind, vec![4, 5]).expect("revision");
    let sqlite_revision = sqlite.put(revision.clone()).expect("sqlite revision");
    let memory_revision = memory.put(revision).expect("memory revision");
    assert_eq!(sqlite_revision, memory_revision);
    let entity = EntityId::new([9; 16]);
    let base = scriberig_store::Snapshot::empty().id();
    let sqlite_snapshot = sqlite
        .apply_changes(base, &[SnapshotChange::Set(entity, sqlite_revision)])
        .expect("sqlite snapshot");
    let memory_snapshot = memory
        .apply_changes(base, &[SnapshotChange::Set(entity, memory_revision)])
        .expect("memory snapshot");
    assert_eq!(sqlite_snapshot, memory_snapshot);
}

/// Verifies committed revisions, snapshots, and roots survive reopening.
#[test]
fn committed_state_survives_reopen() {
    let path = temporary_database_path("reopen");
    let kind = ObjectType::new("test.reopen").expect("type");
    let revision = Revision::new(EncodingVersion::V1, kind, vec![7]).expect("revision");
    let entity = EntityId::new([8; 16]);
    let root = RootName::new("main").expect("root");
    let (revision_id, snapshot) = {
        let store = SqliteStore::open(&path).expect("open");
        let revision_id = store.put(revision).expect("put");
        let snapshot = store
            .apply_changes(
                scriberig_store::Snapshot::empty().id(),
                &[SnapshotChange::Set(entity, revision_id)],
            )
            .expect("snapshot");
        store
            .compare_and_set_root(root.clone(), None, snapshot)
            .expect("root");
        (revision_id, snapshot)
    };
    let store = SqliteStore::open(&path).expect("reopen");
    assert!(store.get(revision_id).expect("get").is_some());
    assert_eq!(store.resolve(snapshot, entity), Ok(Some(revision_id)));
    assert_eq!(store.read_root(&root), Some(snapshot));
    drop(store);
    fs::remove_file(path).expect("cleanup");
}

/// Verifies stale atomic promotion rolls back the attempted snapshot and root.
#[test]
fn failed_atomic_promotion_preserves_previous_root() {
    let store = SqliteStore::open_in_memory().expect("open");
    let root = RootName::new("main").expect("root");
    let base = scriberig_store::Snapshot::empty().id();
    store
        .compare_and_set_root(root.clone(), None, base)
        .expect("base root");
    let kind = ObjectType::new("test.promotion").expect("type");
    let revision = store
        .put(Revision::new(EncodingVersion::V1, kind, vec![9]).expect("revision"))
        .expect("put");
    let entity = EntityId::new([3; 16]);
    let attempted = scriberig_store::Snapshot::empty()
        .apply([SnapshotChange::Set(entity, revision)])
        .id();
    let error = store
        .apply_changes_and_promote(
            root.clone(),
            None,
            base,
            &[SnapshotChange::Set(entity, revision)],
        )
        .expect_err("stale promotion");
    assert!(matches!(
        error,
        scriberig_store_sqlite::PromotionError::Root(_)
    ));
    assert_eq!(store.read_root(&root), Some(base));
    assert!(store.resolve(attempted, entity).is_err());
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
