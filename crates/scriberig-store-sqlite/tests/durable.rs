//! Exercises durable SQLite storage contracts.
use scriberig_core::{EncodingVersion, EntityId, ObjectType, RevisionId, RootName};
use scriberig_store::{Revision, RevisionStore, RootStore, SnapshotChange, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Verifies revisions, snapshots, and root CAS work in SQLite.
#[test]
fn sqlite_round_trip_and_stale_cas() {
    let store = SqliteStore::open_in_memory().expect("open");
    let kind = ObjectType::new("test").expect("type");
    let revision = Revision::new(EncodingVersion::new(1), kind, vec![1, 2]);
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

/// Verifies an absent root can be read after fresh initialization.
#[test]
fn sqlite_starts_without_roots() {
    let store = SqliteStore::open_in_memory().expect("open");
    let root = RootName::new("main").expect("root");
    assert_eq!(store.read_root(&root), None);
    assert!(!store.contains(RevisionId::new([4; 32])).expect("contains"));
}
