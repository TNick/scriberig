//! Tests the in-memory storage backend contracts.

use crate::{MemoryRevisionStore, MemoryRootStore, MemoryStore};
use scriberig_core::{EntityId, RevisionId, RootName, SnapshotId};
use scriberig_store::{CasOutcome, RootStore, SnapshotChange, SnapshotStore};

/// Confirms named roots support atomic stale detection.
#[test]
fn root_store_cas_is_atomic() {
    let store = MemoryRootStore::new();
    let name = RootName::new("main").expect("root");
    let first = SnapshotId::new([1; 32]);
    let second = SnapshotId::new([2; 32]);
    assert_eq!(
        store.compare_and_swap(name.clone(), None, first),
        Ok(CasOutcome::Updated)
    );
    assert_eq!(
        store.compare_and_swap(name.clone(), None, second),
        Ok(CasOutcome::Stale {
            actual: Some(first)
        })
    );
    assert_eq!(store.read_root(&name), Some(first));
}

/// Confirms snapshots are immutable and equivalent maps converge.
#[test]
fn snapshots_branch_and_converge() {
    let store = MemoryStore::new();
    let base = store.empty_snapshot();
    let first = EntityId::new([1; 16]);
    let second = EntityId::new([2; 16]);
    let revision = RevisionId::new([3; 32]);
    let changed = store
        .apply_changes(base, &[SnapshotChange::Set(first, revision)])
        .expect("snapshot");
    let branch = store
        .apply_changes(base, &[SnapshotChange::Set(second, revision)])
        .expect("snapshot");
    assert_ne!(changed, branch);
    assert_eq!(store.resolve(base, first), Ok(None));
    let reordered = store
        .apply_changes(base, &[SnapshotChange::Set(first, revision)])
        .expect("snapshot");
    assert_eq!(changed, reordered);
}

/// Runs the shared backend-neutral conformance suite.
#[test]
fn shared_conformance_passes() {
    scriberig_store::run_core_conformance(&MemoryStore::new()).expect("conformance");
}

/// Verifies observability reports counts without changing immutable state.
#[test]
fn observability_reports_authoritative_counts() {
    use scriberig_store::StoreObservability;
    let store = MemoryStore::new();
    let before = store.empty_snapshot();
    let stats = store.stats().expect("stats");
    assert_eq!(stats.snapshots, Some(1));
    assert_eq!(store.empty_snapshot(), before);
}

/// Confirms duplicate revision insertion is idempotent.
#[test]
fn revisions_are_idempotent() {
    use scriberig_core::{EncodingVersion, ObjectType};
    use scriberig_store::{Revision, RevisionStore};
    let store = MemoryRevisionStore::new();
    let kind = ObjectType::new("test").expect("type");
    let revision = Revision::new(EncodingVersion::new(1), kind, vec![1, 2]);
    let id = store.put(revision.clone()).expect("put");
    assert_eq!(store.put(revision), Ok(id));
}
