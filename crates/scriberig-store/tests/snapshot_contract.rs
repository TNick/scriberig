//! Verifies snapshot convergence and persistent ordered-collection behavior.

use scriberig_core::{
    CanonicalEncode, CanonicalWriter, EncodingVersion, EntityId, ObjectType, RevisionId,
};
use scriberig_store::Revision;
use scriberig_store::{Collection, Snapshot};

/// Confirms update order does not affect a complete snapshot identity.
#[test]
fn equivalent_update_orders_converge() {
    let first = EntityId::new([1; 16]);
    let second = EntityId::new([2; 16]);
    let first_revision = RevisionId::new([3; 32]);
    let second_revision = RevisionId::new([4; 32]);

    let left = Snapshot::empty()
        .with_revision(first, first_revision)
        .with_revision(second, second_revision);
    let right = Snapshot::empty()
        .with_revision(second, second_revision)
        .with_revision(first, first_revision);

    assert_eq!(left.id(), right.id());
}

/// Confirms removing an entity does not mutate its predecessor.
#[test]
fn removing_entity_preserves_prior_snapshot() {
    let entity = EntityId::new([5; 16]);
    let revision = RevisionId::new([6; 32]);
    let base = Snapshot::empty().with_revision(entity, revision);
    let removed = base.without_entity(entity);

    assert_eq!(base.resolve(entity), Some(revision));
    assert_eq!(removed.resolve(entity), None);
    assert_eq!(removed.id(), Snapshot::empty().id());
}

/// Confirms ordered collection reordering preserves child identities.
#[test]
fn reordered_collection_changes_order_only() {
    let first = EntityId::new([7; 16]);
    let second = EntityId::new([8; 16]);
    let original = Collection::create().replace([first, second]);
    let reordered = original.replace([second, first]);

    assert_ne!(original, reordered);
    assert_eq!(
        original.iter().copied().collect::<Vec<_>>(),
        vec![first, second]
    );
    assert_eq!(
        reordered.iter().copied().collect::<Vec<_>>(),
        vec![second, first]
    );
}

/// Confirms reordering changes a container revision but not child identities.
#[test]
fn reordered_collection_changes_container_revision_only() {
    let first = RevisionId::new([9; 32]);
    let second = RevisionId::new([10; 32]);
    let original = Collection::create().replace([first, second]);
    let reordered = original.replace([second, first]);
    let object_type = ObjectType::new("test.container").expect("object type");

    let revision_for = |collection: &Collection<RevisionId>| {
        let mut writer = CanonicalWriter::new();
        collection.encode_canonical(&mut writer);
        Revision::new(
            EncodingVersion::V1,
            object_type.clone(),
            writer.into_bytes(),
        )
        .expect("container revision")
    };

    let original_revision = revision_for(&original);
    let reordered_revision = revision_for(&reordered);

    assert_ne!(original_revision.id(), reordered_revision.id());
    assert_eq!(
        original.iter().copied().collect::<Vec<_>>(),
        [first, second]
    );
    assert_eq!(
        reordered.iter().copied().collect::<Vec<_>>(),
        [second, first]
    );
}

/// Confirms moving one member preserves all unchanged child identities.
#[test]
fn moving_collection_member_preserves_child_identities() {
    let first = RevisionId::new([11; 32]);
    let second = RevisionId::new([12; 32]);
    let third = RevisionId::new([13; 32]);
    let collection = Collection::create().replace([first, second, third]);

    assert_eq!(
        collection
            .move_item(0, 2)
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [second, third, first]
    );
}
