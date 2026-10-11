//! Covers backend-independent semantic change behavior.

use scriberig_core::{EncodingVersion, EntityId, ObjectType};
use scriberig_store::{Revision, SnapshotChange, SnapshotStore};
use scriberig_store_memory::MemoryStore;
use sr_change::compare;

/// Verifies identical snapshots produce no deltas.
#[test]
fn identical_snapshots_are_empty() {
    let store = MemoryStore::default();
    let snapshot = scriberig_store::Snapshot::empty().id();
    let diff = compare(&store, snapshot, snapshot).expect("diff");
    assert_eq!(diff.deltas.len(), 0);
}

/// Verifies arbitrary sibling snapshots classify state changes deterministically.
#[test]
fn sibling_snapshots_report_sorted_entity_deltas_and_summary() {
    let store = MemoryStore::default();
    let kind = ObjectType::new("test").expect("kind");
    let first = Revision::new(EncodingVersion::new(1), kind.clone(), vec![1]).id();
    let second = Revision::new(EncodingVersion::new(1), kind.clone(), vec![2]).id();
    let third = Revision::new(EncodingVersion::new(1), kind, vec![3]).id();
    let base = scriberig_store::Snapshot::empty().id();
    let left = store
        .apply_changes(base, &[SnapshotChange::Set(EntityId::new([2; 16]), first)])
        .expect("left");
    let right = store
        .apply_changes(
            base,
            &[
                SnapshotChange::Set(EntityId::new([1; 16]), second),
                SnapshotChange::Set(EntityId::new([2; 16]), third),
            ],
        )
        .expect("right");
    let diff = compare(&store, left, right).expect("diff");
    assert_eq!(diff.deltas.len(), 2);
    assert_eq!(diff.deltas[0].entity, EntityId::new([1; 16]));
    assert_eq!(diff.deltas[1].entity, EntityId::new([2; 16]));
    assert_eq!(diff.summary().total_changed(), 2);
    assert_eq!(diff.summary().introduced, 1);
    assert_eq!(diff.summary().modified, 1);
}
