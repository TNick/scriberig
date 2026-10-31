//! Verifies atomic snapshot promotion failures leave the prior root intact.

use super::*;

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
    let entity = EntityId::from_bytes([3; 16]);
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
