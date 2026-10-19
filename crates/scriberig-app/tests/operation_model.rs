//! Verifies tracked operation records and structured application failures.

use scriberig_app::{
    ApplicationBackend, ApplicationBackendComposition, ApplicationContext, ApplicationError,
    ApplicationRequest, ApplicationResult, ConflictDetails, DiffRequest, HistoryRequest,
    OperationId, OperationKind, OperationProvider, OperationRecord, OperationStatus,
    SnapshotSelector, StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::{EntityId, RevisionId, RootName, SnapshotId};
use scriberig_store::{RootStore, SnapshotChange, SnapshotStore};
use scriberig_store_memory::MemoryStore;
use sr_change::{Migration, Provenance, SourceKind};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Verifies terminal operation states cannot be changed.
#[test]
fn terminal_operation_is_immutable() {
    let operation = OperationRecord::queued(
        OperationId::new("inspect-1").expect("operation"),
        OperationKind::Inspect,
    )
    .transition(OperationStatus::Running)
    .expect("running")
    .transition(OperationStatus::Completed)
    .expect("completed");
    let error = operation
        .transition(OperationStatus::Failed)
        .expect_err("terminal transition");
    assert!(matches!(error, ApplicationError::OperationFailed { .. }));
}

/// Verifies conflict and ambiguity errors retain semantic handles.
#[test]
fn structured_failures_preserve_handles() {
    let base = RevisionId::new([1; 32]);
    let current = RevisionId::new([2; 32]);
    let snapshot = SnapshotId::new([3; 32]);
    let conflict = ApplicationError::Conflict {
        subject: "entity_1".to_owned(),
        details: Box::new(ConflictDetails {
            base_revision: Some(base),
            current_revision: Some(current),
            ..ConflictDetails::default()
        }),
    };
    assert_eq!(conflict.category(), "conflict");
    let ambiguous = ApplicationError::Ambiguous {
        scope: snapshot.to_string(),
        candidates: vec!["entity_1".to_owned(), "entity_2".to_owned()],
        reason: "duplicate semantic names".to_owned(),
    };
    assert_eq!(ambiguous.category(), "ambiguous");
}

/// Verifies storage-backed diff execution preserves exact snapshot identity.
#[test]
fn storage_diff_is_snapshot_bound() {
    let store = MemoryStore::new();
    let snapshot = store.empty_snapshot();
    let provider = StoreOperationProvider::new(store);
    let result = provider
        .diff(
            &ApplicationContext::empty(),
            &DiffRequest {
                from: snapshot,
                to: snapshot,
            },
        )
        .expect("diff");
    assert_eq!(result.snapshot, Some(snapshot));
    assert!(result.handle.starts_with("diff:"));
    assert_eq!(result.diff.as_ref().map(|diff| diff.left), Some(snapshot));
}

/// Verifies explicit candidate merge combines disjoint changes without promotion.
#[test]
fn storage_merge_combines_disjoint_candidates_without_promoting_root() {
    let store = MemoryStore::new();
    let base = store.empty_snapshot();
    let left_entity = EntityId::from_bytes([1; 16]);
    let right_entity = EntityId::from_bytes([2; 16]);
    let left = store
        .apply_changes(
            base,
            &[SnapshotChange::Set(left_entity, RevisionId::new([3; 32]))],
        )
        .expect("left candidate");
    let right = store
        .apply_changes(
            base,
            &[SnapshotChange::Set(right_entity, RevisionId::new([4; 32]))],
        )
        .expect("right candidate");
    let provider = StoreOperationProvider::new(store);
    let merged = provider
        .merge(
            &ApplicationContext::empty(),
            &scriberig_app::MergeRequest { base, left, right },
        )
        .expect("merge");
    assert_eq!(merged.conflicts, Vec::new());
    assert_eq!(merged.resolved_deltas.len(), 2);
    assert!(merged.result.is_some());
    assert!(
        provider
            .store()
            .read_root(&RootName::new("main").expect("root"))
            .is_none()
    );
}

/// Verifies a same-target candidate conflict remains structured and unresolved.
#[test]
fn storage_merge_returns_same_target_conflict_without_promotion() {
    let store = MemoryStore::new();
    let base = store.empty_snapshot();
    let entity = EntityId::from_bytes([1; 16]);
    let left = store
        .apply_changes(
            base,
            &[SnapshotChange::Set(entity, RevisionId::new([3; 32]))],
        )
        .expect("left candidate");
    let right = store
        .apply_changes(
            base,
            &[SnapshotChange::Set(entity, RevisionId::new([4; 32]))],
        )
        .expect("right candidate");
    let provider = StoreOperationProvider::new(store);
    let merged = provider
        .merge(
            &ApplicationContext::empty(),
            &scriberig_app::MergeRequest { base, left, right },
        )
        .expect("merge");
    assert_eq!(merged.conflicts.len(), 1);
    assert!(merged.result.is_none());
    assert!(
        provider
            .store()
            .read_root(&RootName::new("main").expect("root"))
            .is_none()
    );
}

/// Verifies operation providers can share a storage backend through `Arc`.
#[test]
fn storage_operation_provider_accepts_shared_store() {
    let store = Arc::new(MemoryStore::new());
    let snapshot = store.empty_snapshot();
    let provider = StoreOperationProvider::new(Arc::clone(&store));
    let result = provider
        .diff(
            &ApplicationContext::empty(),
            &DiffRequest {
                from: snapshot,
                to: snapshot,
            },
        )
        .expect("diff");
    assert_eq!(result.snapshot, Some(snapshot));
}

/// Verifies read and operation providers can be composed over one store.
#[test]
fn application_backend_composition_dispatches_storage_diff() {
    let store = Arc::new(MemoryStore::new());
    let snapshot = store.empty_snapshot();
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let result = backend
        .execute(
            &ApplicationContext::empty(),
            &ApplicationRequest::Diff(DiffRequest {
                from: snapshot,
                to: snapshot,
            }),
        )
        .expect("composed diff");
    assert!(matches!(result, ApplicationResult::Diff(_)));
}

/// Verifies history execution retains the exact requested snapshot.
#[test]
fn storage_history_is_snapshot_bound() {
    let store = MemoryStore::new();
    let snapshot = store.empty_snapshot();
    let provider = StoreOperationProvider::new(store).with_history(sr_change::HistoryGraph::new());
    let result = provider
        .history(
            &ApplicationContext::empty(),
            &HistoryRequest {
                snapshot: SnapshotSelector::Exact(snapshot),
            },
        )
        .expect("history");
    assert_eq!(result.snapshot, Some(snapshot));
    assert_eq!(result.history.as_ref().map(Vec::len), Some(0));
}

/// Verifies history is derived from durable snapshot ancestry when no graph is
/// explicitly injected by the caller.
#[test]
fn storage_history_derives_snapshot_transition() {
    let store = MemoryStore::new();
    let base = store.empty_snapshot();
    let updated = EntityId::from_bytes([1; 16]);
    let removed = EntityId::from_bytes([2; 16]);
    let created = EntityId::from_bytes([3; 16]);
    let old_revision = RevisionId::new([4; 32]);
    let updated_revision = RevisionId::new([5; 32]);
    let removed_revision = RevisionId::new([6; 32]);
    let populated = store
        .apply_changes(
            base,
            &[
                SnapshotChange::Set(updated, old_revision),
                SnapshotChange::Set(removed, removed_revision),
            ],
        )
        .expect("populated base");
    let snapshot = store
        .apply_changes(
            populated,
            &[
                SnapshotChange::Set(updated, updated_revision),
                SnapshotChange::Remove(removed),
                SnapshotChange::Set(created, RevisionId::new([7; 32])),
            ],
        )
        .expect("snapshot");

    let result = StoreOperationProvider::new(store)
        .history(
            &ApplicationContext::empty(),
            &HistoryRequest {
                snapshot: SnapshotSelector::Exact(snapshot),
            },
        )
        .expect("derived history");

    let history = result.history.expect("history rows");
    let transition = history
        .iter()
        .find(|transition| transition.result == snapshot)
        .expect("requested snapshot transition");
    assert_eq!(transition.base, populated);
    let deltas = transition
        .deltas
        .iter()
        .map(|delta| (delta.entity, (delta.before, delta.after)))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        deltas.get(&updated),
        Some(&(Some(old_revision), Some(updated_revision)))
    );
    assert_eq!(deltas.get(&removed), Some(&(Some(removed_revision), None)));
    assert_eq!(
        deltas.get(&created),
        Some(&(None, Some(RevisionId::new([7; 32]))))
    );
}

/// Verifies apply is candidate-only while commit performs root promotion.
#[test]
fn storage_apply_and_commit_keep_promotion_explicit() {
    let store = MemoryStore::new();
    let snapshot = store.empty_snapshot();
    let migration = Migration::new(
        snapshot,
        snapshot,
        Vec::new(),
        Vec::new(),
        Provenance {
            source: SourceKind::User,
            actor: None,
            workspace: None,
            projection: None,
            validation: Vec::new(),
            parents: Vec::new(),
            metadata: BTreeMap::new(),
        },
    )
    .expect("migration");
    let provider = StoreOperationProvider::new(store);
    assert!(provider.register_migration("empty", migration));
    let applied = provider
        .apply(
            &ApplicationContext::empty(),
            &scriberig_app::ApplyRequest {
                base_snapshot: snapshot,
                branch: None,
                change: "empty".to_owned(),
            },
        )
        .expect("apply");
    assert_eq!(applied.snapshot, Some(snapshot));
    assert!(
        provider
            .store()
            .read_root(&RootName::new("main").expect("root"))
            .is_none()
    );
    provider
        .store()
        .set_root(RootName::new("main").expect("root"), snapshot)
        .expect("seed root");
    let committed = provider
        .commit(
            &ApplicationContext::empty(),
            &scriberig_app::CommitRequest {
                base_snapshot: snapshot,
                branch: None,
                change: "empty".to_owned(),
            },
        )
        .expect("commit");
    assert_eq!(committed.snapshot, Some(snapshot));
}

/// Verifies registered migrations remain readable as exact proposal details.
#[test]
fn storage_migration_detail_is_snapshot_bound() {
    let store = MemoryStore::new();
    let base = store.empty_snapshot();
    let result = SnapshotId::new([9; 32]);
    let migration = Migration::new(
        base,
        result,
        vec![
            sr_change::EntityDelta::new(
                scriberig_core::EntityId::from_bytes([1; 16]),
                Some(RevisionId::new([2; 32])),
                Some(RevisionId::new([3; 32])),
            )
            .expect("delta"),
        ],
        Vec::new(),
        Provenance {
            source: SourceKind::User,
            actor: None,
            workspace: None,
            projection: None,
            validation: Vec::new(),
            parents: Vec::new(),
            metadata: BTreeMap::new(),
        },
    )
    .expect("migration");
    let provider = StoreOperationProvider::new(store);
    assert!(provider.register_migration("proposal-1", migration));
    let detail = provider
        .migration_detail("proposal-1")
        .expect("migration detail");
    assert_eq!(detail.base, base);
    assert_eq!(detail.result, result);
    assert_eq!(detail.deltas.len(), 1);
}

/// Verifies branch creation persists the requested named root.
#[test]
fn storage_branch_creates_requested_root() {
    let store = MemoryStore::new();
    let snapshot = store.empty_snapshot();
    let provider = StoreOperationProvider::new(store);
    let branch = RootName::new("feature").expect("branch");
    provider
        .branch(
            &ApplicationContext::empty(),
            &scriberig_app::BranchRequest {
                base: SnapshotSelector::Exact(snapshot),
                name: branch.clone(),
            },
        )
        .expect("branch");
    assert_eq!(provider.store().read_root(&branch), Some(snapshot));
}
