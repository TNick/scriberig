//! Exercises backend-neutral maintenance behavior in memory storage.

use scriberig_core::{EncodingVersion, EntityId, ObjectType};
use scriberig_store::{
    MaintenanceStore, ProtectedRoots, Revision, RevisionStore, SnapshotChange, SnapshotStore,
};
use scriberig_store_memory::MemoryStore;

/// Verifies memory maintenance distinguishes protected and abandoned objects.
#[test]
fn memory_maintenance_marks_and_sweeps_unreachable_objects() {
    let store = MemoryStore::new();
    let revision = store
        .put(
            Revision::new(
                EncodingVersion::V1,
                ObjectType::new("maintenance.item").expect("type"),
                vec![1],
            )
            .expect("revision"),
        )
        .expect("put");
    let snapshot = store
        .apply_changes(
            store.empty_snapshot(),
            &[SnapshotChange::Set(EntityId::new([1; 16]), revision)],
        )
        .expect("snapshot");
    let report = store
        .gc_dry_run(&ProtectedRoots {
            snapshots: [snapshot].into_iter().collect(),
        })
        .expect("dry run");
    assert_eq!(report.corrupt, []);
    assert!(!report.unreachable.snapshots.contains(&snapshot));
    assert!(!report.unreachable.revisions.contains(&revision));
    assert!(store.rebuild_indexes().expect("indexes").consistent);
}
