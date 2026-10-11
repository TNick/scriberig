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

use scriberig_core::RevisionId;
use scriberig_store::Snapshot;
use sr_change::{ConflictKind, MergeInput, merge_snapshots};

/// Verifies disjoint branch changes resolve without conflicts.
#[test]
fn disjoint_changes_merge_automatically() {
    let base = Snapshot::empty();
    let left = base.with_revision(EntityId::new([1; 16]), RevisionId::new([2; 32]));
    let right = base.with_revision(EntityId::new([3; 16]), RevisionId::new([4; 32]));
    let result = merge_snapshots(
        MergeInput {
            base: base.id(),
            left: left.id(),
            right: right.id(),
        },
        &base,
        &left,
        &right,
        None,
    )
    .expect("merge");
    assert_eq!(result.resolved_deltas.len(), 2);
    assert_eq!(result.conflicts.len(), 0);
}

/// Verifies competing modifications remain structured conflicts.
#[test]
fn competing_changes_preserve_unrelated_resolution() {
    let entity = EntityId::new([1; 16]);
    let other = EntityId::new([3; 16]);
    let base = Snapshot::empty().with_revision(entity, RevisionId::new([2; 32]));
    let left = base
        .with_revision(entity, RevisionId::new([5; 32]))
        .with_revision(other, RevisionId::new([6; 32]));
    let right = base.with_revision(entity, RevisionId::new([7; 32]));
    let result = merge_snapshots(
        MergeInput {
            base: base.id(),
            left: left.id(),
            right: right.id(),
        },
        &base,
        &left,
        &right,
        None,
    )
    .expect("merge");
    assert_eq!(result.conflicts[0].kind, ConflictKind::BothModified);
    assert_eq!(result.resolved_deltas.len(), 1);
    assert_eq!(result.resolved_deltas[0].entity, other);
}

/// Verifies delete/modify and competing-create classifications.
#[test]
fn delete_modify_and_competing_create_are_distinct() {
    let entity = EntityId::new([1; 16]);
    let base = Snapshot::empty().with_revision(entity, RevisionId::new([2; 32]));
    let left = base.without_entity(entity);
    let right = base.with_revision(entity, RevisionId::new([3; 32]));
    let delete_modify = merge_snapshots(
        MergeInput {
            base: base.id(),
            left: left.id(),
            right: right.id(),
        },
        &base,
        &left,
        &right,
        None,
    )
    .expect("merge");
    assert_eq!(delete_modify.conflicts[0].kind, ConflictKind::DeleteModify);
    let empty = Snapshot::empty();
    let created_left = empty.with_revision(entity, RevisionId::new([8; 32]));
    let created_right = empty.with_revision(entity, RevisionId::new([9; 32]));
    let competing = merge_snapshots(
        MergeInput {
            base: empty.id(),
            left: created_left.id(),
            right: created_right.id(),
        },
        &empty,
        &created_left,
        &created_right,
        None,
    )
    .expect("merge");
    assert_eq!(competing.conflicts[0].kind, ConflictKind::CompetingCreate);
}
