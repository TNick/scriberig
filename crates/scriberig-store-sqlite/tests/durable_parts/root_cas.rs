//! Verifies root promotion serializes across independent SQLite connections.

use std::sync::{Arc, Barrier};

use scriberig_core::{EncodingVersion, EntityId, ObjectType, RootName};
use scriberig_store::{Revision, RootStore, Snapshot, SnapshotChange};
use scriberig_store_sqlite::{PromotionError, SqliteStore};

/// Verifies independent SQLite handles report a stale-root conflict on CAS.
#[test]
fn independent_handles_serialize_root_compare_and_swap() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-independent-cas-{}.db",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    let setup = SqliteStore::open(&path).expect("open setup store");
    let root = RootName::new("independent-cas").expect("root");
    let base = Snapshot::empty().id();
    setup
        .compare_and_set_root(root.clone(), None, base)
        .expect("set initial root");
    drop(setup);

    let barrier = Arc::new(Barrier::new(2));
    let stores = [
        SqliteStore::open(&path).expect("open first competing store"),
        SqliteStore::open(&path).expect("open second competing store"),
    ];
    let handles = stores
        .into_iter()
        .zip([1_u8, 2_u8])
        .map(|(store, value)| {
            let root = root.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let entity = EntityId::from_bytes([value; 16]);
                let revision = Revision::new(
                    EncodingVersion::V1,
                    ObjectType::new("test.independent-cas").expect("type"),
                    vec![value],
                )
                .expect("revision");
                let revision_id = revision.id();
                barrier.wait();
                store.apply_revisions_and_promote(
                    root,
                    Some(base),
                    base,
                    &[revision],
                    &[SnapshotChange::Set(entity, revision_id)],
                )
            })
        })
        .collect::<Vec<_>>();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().expect("CAS worker"))
        .collect::<Vec<_>>();

    assert_eq!(
        results.iter().filter(|result| result.is_ok()).count(),
        1,
        "one independent writer must win"
    );
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(PromotionError::Root(_))))
            .count(),
        1,
        "the losing writer must receive a structured root conflict"
    );
    let _ = std::fs::remove_file(path);
}
