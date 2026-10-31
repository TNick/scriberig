//! Covers backend-independent semantic change behavior.

use scriberig_core::{EncodingVersion, EntityId, ObjectType};
use scriberig_store::{
    Revision, SnapshotChange, SnapshotRelation, SnapshotRelationChange, SnapshotStore,
};
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
    let first = Revision::new(EncodingVersion::new(1), kind.clone(), vec![1])
        .expect("revision")
        .id();
    let second = Revision::new(EncodingVersion::new(1), kind.clone(), vec![2])
        .expect("revision")
        .id();
    let third = Revision::new(EncodingVersion::new(1), kind, vec![3])
        .expect("revision")
        .id();
    let base = scriberig_store::Snapshot::empty().id();
    let left = store
        .apply_changes(
            base,
            &[SnapshotChange::Set(EntityId::from_bytes([2; 16]), first)],
        )
        .expect("left");
    let right = store
        .apply_changes(
            base,
            &[
                SnapshotChange::Set(EntityId::from_bytes([1; 16]), second),
                SnapshotChange::Set(EntityId::from_bytes([2; 16]), third),
            ],
        )
        .expect("right");
    let diff = compare(&store, left, right).expect("diff");
    assert_eq!(diff.deltas.len(), 2);
    assert_eq!(diff.deltas[0].entity, EntityId::from_bytes([1; 16]));
    assert_eq!(diff.deltas[1].entity, EntityId::from_bytes([2; 16]));
    assert_eq!(diff.summary().total_changed(), 2);
    assert_eq!(diff.summary().introduced, 1);
    assert_eq!(diff.summary().modified, 1);
}

use scriberig_core::RevisionId;
use scriberig_store::Snapshot;
use sr_change::{ConflictKind, MergeInput, merge_snapshots, merge_snapshots_into_store};

/// Verifies disjoint branch changes resolve without conflicts.
#[test]
fn disjoint_changes_merge_automatically() {
    let base = Snapshot::empty();
    let left = base.with_revision(EntityId::from_bytes([1; 16]), RevisionId::new([2; 32]));
    let right = base.with_revision(EntityId::from_bytes([3; 16]), RevisionId::new([4; 32]));
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

/// Verifies a conflict-free merge is materialized as a retained snapshot.
#[test]
fn disjoint_merge_creates_result_snapshot() {
    let store = MemoryStore::default();
    let empty = store.empty_snapshot();
    let prior_relation = SnapshotRelation::new(
        EntityId::from_bytes([19; 16]),
        ObjectType::new("project.mentions").expect("relation type"),
        EntityId::from_bytes([18; 16]),
    );
    let base = store
        .apply_relation_changes(
            empty,
            &[SnapshotRelationChange::Add(prior_relation.clone())],
        )
        .expect("base relation");
    let left = store
        .apply_changes(
            base,
            &[SnapshotChange::Set(
                EntityId::from_bytes([1; 16]),
                RevisionId::new([2; 32]),
            )],
        )
        .expect("left");
    let right = store
        .apply_changes(
            base,
            &[SnapshotChange::Set(
                EntityId::from_bytes([3; 16]),
                RevisionId::new([4; 32]),
            )],
        )
        .expect("right");
    let result =
        merge_snapshots_into_store(&store, MergeInput { base, left, right }, None).expect("merge");
    let merged = result.result.expect("merged snapshot");
    assert_eq!(
        store
            .resolve(merged, EntityId::from_bytes([1; 16]))
            .expect("resolve"),
        Some(RevisionId::new([2; 32]))
    );
    assert_eq!(
        store
            .resolve(merged, EntityId::from_bytes([3; 16]))
            .expect("resolve"),
        Some(RevisionId::new([4; 32]))
    );
    assert_eq!(
        store.relations(merged).expect("preserved relation"),
        vec![prior_relation]
    );
}

/// Verifies a merge retains relation additions from both branches atomically.
#[test]
fn disjoint_relation_changes_merge_into_the_retained_snapshot() {
    let store = MemoryStore::default();
    let empty = store.empty_snapshot();
    let prior_relation = SnapshotRelation::new(
        EntityId::from_bytes([19; 16]),
        ObjectType::new("project.mentions").expect("relation type"),
        EntityId::from_bytes([18; 16]),
    );
    let base = store
        .apply_relation_changes(
            empty,
            &[SnapshotRelationChange::Add(prior_relation.clone())],
        )
        .expect("base relation");
    let first_relation = SnapshotRelation::new(
        EntityId::from_bytes([20; 16]),
        ObjectType::new("document.references").expect("relation type"),
        EntityId::from_bytes([21; 16]),
    );
    let second_relation = SnapshotRelation::new(
        EntityId::from_bytes([22; 16]),
        ObjectType::new("session.touches").expect("relation type"),
        EntityId::from_bytes([23; 16]),
    );
    let left = store
        .apply_relation_changes(
            base,
            &[
                SnapshotRelationChange::Remove(prior_relation),
                SnapshotRelationChange::Add(first_relation.clone()),
            ],
        )
        .expect("left relation");
    let right = store
        .apply_relation_changes(
            base,
            &[SnapshotRelationChange::Add(second_relation.clone())],
        )
        .expect("right relation");

    let result =
        merge_snapshots_into_store(&store, MergeInput { base, left, right }, None).expect("merge");
    let merged = result.result.expect("merged snapshot");
    assert_eq!(
        store.relations(merged).expect("merged relations"),
        vec![first_relation, second_relation]
    );
}

/// Verifies competing modifications remain structured conflicts.
#[test]
fn competing_changes_preserve_unrelated_resolution() {
    let entity = EntityId::from_bytes([1; 16]);
    let other = EntityId::from_bytes([3; 16]);
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
    let entity = EntityId::from_bytes([1; 16]);
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

/// Verifies creation and removal remain explicit semantic deltas.
#[test]
fn creation_and_removal_are_classified_without_text_markers() {
    let entity = EntityId::from_bytes([9; 16]);
    let store = MemoryStore::default();
    let base = scriberig_store::Snapshot::empty().id();
    let created = store
        .apply_changes(
            base,
            &[
                SnapshotChange::Set(entity, RevisionId::new([1; 32])),
                SnapshotChange::Set(EntityId::from_bytes([8; 16]), RevisionId::new([2; 32])),
            ],
        )
        .expect("create");
    let removed = store
        .apply_changes(created, &[SnapshotChange::Remove(entity)])
        .expect("remove");
    let diff = compare(&store, created, removed).expect("diff");
    assert!(diff.deltas.iter().any(sr_change::EntityDelta::is_removed));
}

/// Verifies convergent edits merge without choosing one side as a conflict.
#[test]
fn convergent_edits_are_conflict_free() {
    let entity = EntityId::from_bytes([4; 16]);
    let base = scriberig_store::Snapshot::empty();
    let left = base.with_revision(entity, RevisionId::new([7; 32]));
    let right = base.with_revision(entity, RevisionId::new([7; 32]));
    let result = sr_change::merge_snapshots(
        sr_change::MergeInput {
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
    assert_eq!(result.conflicts.len(), 0);
}

/// Verifies representation-only reuse does not manufacture a semantic migration.
#[test]
fn identical_canonical_state_has_no_migration() {
    let entity = EntityId::from_bytes([5; 16]);
    let revision = RevisionId::new([6; 32]);
    let first = scriberig_store::Snapshot::empty().with_revision(entity, revision);
    let second = scriberig_store::Snapshot::empty().with_revision(entity, revision);
    let store = MemoryStore::default();
    let diff = compare(&store, first.id(), second.id()).expect("diff");
    assert_eq!(diff.deltas.len(), 0);
}
