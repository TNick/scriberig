//! Verifies the complete backend-independent persistence scenario.

use scriberig_core::{EncodingVersion, EntityId, ObjectType, RevisionId, RootName, SnapshotId};
use scriberig_store::{
    ProtectedRoots, Revision, RevisionStore, RootError, RootStore, SnapshotChange, SnapshotStore,
    StoreObservability,
};
use scriberig_store_memory::MemoryStore;
use scriberig_store_sqlite::SqliteStore;

/// Records identities produced by one persistence scenario.
///
/// # Private members
/// All members are retained to compare backend results and verify branches.
struct Scenario {
    a1: RevisionId,
    a2: RevisionId,
    b1: RevisionId,
    b2: RevisionId,
    c1: RevisionId,
    s1: SnapshotId,
    s2: SnapshotId,
    s3: SnapshotId,
}

/// Creates canonical revisions and the S1/S2/S3 branch scenario.
///
/// # Parameters
/// - `store`: Backend receiving canonical content.
///
/// # Returns
/// Stable revision and snapshot identities for the scenario.
fn create_scenario<S: RevisionStore + SnapshotStore + RootStore>(store: &S) -> Scenario {
    let kind = ObjectType::new("persistence.item").expect("type");
    let a1 = store
        .put(Revision::new(EncodingVersion::V1, kind.clone(), vec![1]).expect("revision"))
        .expect("A1");
    let a2 = store
        .put(Revision::new(EncodingVersion::V1, kind.clone(), vec![2]).expect("revision"))
        .expect("A2");
    let b1 = store
        .put(Revision::new(EncodingVersion::V1, kind.clone(), vec![3]).expect("revision"))
        .expect("B1");
    let b2 = store
        .put(Revision::new(EncodingVersion::V1, kind.clone(), vec![4]).expect("revision"))
        .expect("B2");
    let c1 = store
        .put(Revision::new(EncodingVersion::V1, kind, vec![5]).expect("revision"))
        .expect("C1");
    let a = EntityId::from_bytes([1; 16]);
    let b = EntityId::from_bytes([2; 16]);
    let c = EntityId::from_bytes([3; 16]);
    let empty = scriberig_store::Snapshot::empty().id();
    let s1 = store
        .apply_changes(
            empty,
            &[
                SnapshotChange::Set(a, a1),
                SnapshotChange::Set(b, b1),
                SnapshotChange::Set(c, c1),
            ],
        )
        .expect("S1");
    let root = RootName::new("main").expect("root");
    store
        .compare_and_set_root(root.clone(), None, s1)
        .expect("initial root");
    let s2 = store
        .apply_changes(s1, &[SnapshotChange::Set(a, a2)])
        .expect("S2");
    let s3 = store
        .apply_changes(s1, &[SnapshotChange::Set(b, b2)])
        .expect("S3");
    store
        .compare_and_set_root(root.clone(), Some(s1), s2)
        .expect("advance root");
    assert_eq!(store.read_root(&root), Some(s2));
    assert_eq!(store.resolve(s1, b), Ok(Some(b1)));
    assert_eq!(store.resolve(s2, b), Ok(Some(b1)));
    assert_eq!(store.resolve(s2, c), Ok(Some(c1)));
    assert_eq!(store.resolve(s3, a), Ok(Some(a1)));
    assert_eq!(store.resolve(s3, c), Ok(Some(c1)));
    assert_ne!(s1, s2);
    assert_ne!(s1, s3);
    assert_eq!(store.parents(s2).expect("S2 parents"), vec![s1]);
    assert_eq!(store.ancestors(s2).expect("S2 ancestry"), vec![empty, s1]);
    Scenario {
        a1,
        a2,
        b1,
        b2,
        c1,
        s1,
        s2,
        s3,
    }
}

/// Verifies branch durability, stale CAS, backend identity, and GC integrity.
#[test]
fn persistence_vertical_slice_is_backend_independent_and_durable() {
    let memory = MemoryStore::new();
    let memory_scenario = create_scenario(&memory);
    let path =
        std::env::temp_dir().join(format!("scriberig-persistence-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let sqlite = SqliteStore::open(&path).expect("open sqlite");
    let sqlite_scenario = create_scenario(&sqlite);
    assert_eq!(memory_scenario.a1, sqlite_scenario.a1);
    assert_eq!(memory_scenario.a2, sqlite_scenario.a2);
    assert_eq!(memory_scenario.b1, sqlite_scenario.b1);
    assert_eq!(memory_scenario.b2, sqlite_scenario.b2);
    assert_eq!(memory_scenario.c1, sqlite_scenario.c1);
    assert_eq!(memory_scenario.s1, sqlite_scenario.s1);
    assert_eq!(memory_scenario.s2, sqlite_scenario.s2);
    assert_eq!(memory_scenario.s3, sqlite_scenario.s3);
    let root = RootName::new("main").expect("root");
    let stale =
        sqlite.compare_and_set_root(root.clone(), Some(sqlite_scenario.s1), sqlite_scenario.s3);
    assert!(
        matches!(stale, Err(RootError::Conflict { actual: Some(actual), .. }) if actual == sqlite_scenario.s2)
    );
    drop(sqlite);
    let reopened = SqliteStore::open(&path).expect("reopen sqlite");
    assert_eq!(reopened.read_root(&root), Some(sqlite_scenario.s2));
    assert_eq!(
        reopened
            .parents(sqlite_scenario.s2)
            .expect("parents after reopen"),
        vec![sqlite_scenario.s1]
    );
    for snapshot in [sqlite_scenario.s1, sqlite_scenario.s2, sqlite_scenario.s3] {
        assert!(reopened.iter(snapshot).is_ok());
    }
    let protected = ProtectedRoots {
        snapshots: [sqlite_scenario.s2].into_iter().collect(),
    };
    let dry_run = reopened.gc_dry_run(&protected).expect("gc dry run");
    assert!(!dry_run.unreachable.snapshots.contains(&sqlite_scenario.s2));
    assert!(dry_run.unreachable.snapshots.contains(&sqlite_scenario.s3));
    let sweep = reopened.gc_sweep(&protected).expect("gc sweep");
    assert_eq!(sweep.corrupt.len(), 0);
    assert!(reopened.iter(sqlite_scenario.s2).is_ok());
    assert!(reopened.iter(sqlite_scenario.s3).is_err());
    assert_eq!(reopened.stats().expect("stats").named_roots, Some(1));
    let _ = std::fs::remove_file(path);
}
