//! Unit tests for SQLite transaction behavior.

use super::*;
use scriberig_core::{EntityId, RevisionId, RootName};
use scriberig_store::{RootStore, SnapshotChange};

/// Verifies an interrupted transaction does not publish prepared content.
#[test]
fn interrupted_promotion_keeps_previous_root() {
    let store = SqliteStore::open_in_memory().expect("open");
    let root = RootName::new("main").expect("root");
    let base = Snapshot::empty().id();
    store
        .compare_and_set_root(root.clone(), None, base)
        .expect("base root");
    let next = Snapshot::empty().apply([SnapshotChange::Set(
        EntityId::from_bytes([1; 16]),
        RevisionId::new([2; 32]),
    )]);
    let mut connection = store.connection.lock().expect("connection");
    let transaction = connection.transaction().expect("transaction");
    SqliteStore::store_snapshot_in_transaction(&transaction, &next).expect("prepare");
    drop(transaction);
    drop(connection);
    assert_eq!(store.read_root(&root), Some(base));
    assert!(store.snapshot(next.id()).is_err());
}
