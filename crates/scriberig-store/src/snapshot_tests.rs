//! Tests private persistent snapshot representation invariants.

use super::{Node, Snapshot};
use scriberig_core::{EntityId, RevisionId};
use std::sync::Arc;

/// Confirms removing the only mapping converges with a fresh empty map.
#[test]
fn removing_the_only_mapping_converges_with_empty() {
    let entity = EntityId::new([1; 16]);
    let snapshot = Snapshot::empty().with_revision(entity, RevisionId::new([2; 32]));

    assert_eq!(snapshot.without_entity(entity).id(), Snapshot::empty().id());
}

/// Confirms an update shares an unaffected top-level trie branch.
#[test]
fn updating_one_branch_shares_unaffected_structure() {
    let first = EntityId::new([0x10; 16]);
    let second = EntityId::new([0x20; 16]);
    let base = Snapshot::empty()
        .with_revision(first, RevisionId::new([1; 32]))
        .with_revision(second, RevisionId::new([2; 32]));
    let updated = base.with_revision(first, RevisionId::new([3; 32]));

    let Node::Branch(base_children) = base.root.as_ref() else {
        panic!("expected a branch root");
    };
    let Node::Branch(updated_children) = updated.root.as_ref() else {
        panic!("expected a branch root");
    };
    assert!(Arc::ptr_eq(
        base_children[2].as_ref().expect("second branch"),
        updated_children[2].as_ref().expect("second branch"),
    ));
}
