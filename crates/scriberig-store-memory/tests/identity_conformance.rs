//! Verifies the integrated identity, snapshot, and root contracts.

use scriberig_core::{EncodingVersion, EntityId, ObjectType, RootName};
use scriberig_store::{
    CasOutcome, Revision, RevisionStore, RootStore, Snapshot, SnapshotChange, SnapshotStore,
};
use scriberig_store_memory::MemoryStore;

/// Exercises stable entities, immutable revisions, snapshots, diffs, and roots.
#[test]
fn identity_subsystem_conformance() {
    let store = MemoryStore::new();
    let kind = ObjectType::new("test.entity").expect("type");
    let function = EntityId::new([1; 16]);
    let first = EntityId::new([2; 16]);
    let second = EntityId::new([3; 16]);
    let branch = EntityId::new([4; 16]);
    let fr7 = store
        .put(Revision::new(EncodingVersion::V1, kind.clone(), vec![7]))
        .expect("fr7");
    let pr3 = store
        .put(Revision::new(EncodingVersion::V1, kind.clone(), vec![3]))
        .expect("pr3");
    let pr8 = store
        .put(Revision::new(EncodingVersion::V1, kind.clone(), vec![8]))
        .expect("pr8");
    let br2 = store
        .put(Revision::new(EncodingVersion::V1, kind.clone(), vec![2]))
        .expect("br2");
    let pr9 = store
        .put(Revision::new(EncodingVersion::V1, kind, vec![9]))
        .expect("pr9");
    let base = store.empty_snapshot();
    let s1 = store
        .apply_changes(
            base,
            &[
                SnapshotChange::Set(function, fr7),
                SnapshotChange::Set(first, pr3),
                SnapshotChange::Set(second, pr8),
                SnapshotChange::Set(branch, br2),
            ],
        )
        .expect("s1");
    let s2 = store
        .apply_changes(s1, &[SnapshotChange::Set(second, pr9)])
        .expect("s2");
    assert_ne!(s1, s2);
    assert_eq!(store.resolve(s2, function), Ok(Some(fr7)));
    assert_eq!(store.resolve(s2, first), Ok(Some(pr3)));
    assert_eq!(store.resolve(s2, branch), Ok(Some(br2)));
    assert_eq!(store.resolve(s1, second), Ok(Some(pr8)));
    assert_eq!(store.resolve(s2, second), Ok(Some(pr9)));
    let map = Snapshot::from_entries(
        [(function, fr7), (first, pr3), (second, pr8), (branch, br2)]
            .into_iter()
            .collect(),
    );
    assert_eq!(map.id(), s1);
    let changes = Snapshot::from_entries(
        [(function, fr7), (first, pr3), (second, pr9), (branch, br2)]
            .into_iter()
            .collect(),
    )
    .diff(&Snapshot::from_entries(
        [(function, fr7), (first, pr3), (second, pr8), (branch, br2)]
            .into_iter()
            .collect(),
    ));
    assert_eq!(changes.len(), 1);
    assert_eq!(store.iter(s1).expect("history").len(), 4);
    let root = RootName::new("main").expect("root");
    assert_eq!(
        store.compare_and_swap(root.clone(), None, s1),
        Ok(CasOutcome::Updated)
    );
    assert_eq!(
        store.compare_and_swap(root.clone(), Some(s1), s2),
        Ok(CasOutcome::Updated)
    );
    assert_eq!(
        store.compare_and_swap(root.clone(), Some(s1), s1),
        Ok(CasOutcome::Stale { actual: Some(s2) })
    );
    assert_eq!(store.read_root(&root), Some(s2));
}
