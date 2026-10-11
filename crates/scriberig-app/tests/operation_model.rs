//! Verifies tracked operation records and structured application failures.

use scriberig_app::{
    ApplicationContext, ApplicationError, ConflictDetails, DiffRequest, HistoryRequest,
    OperationId, OperationKind, OperationProvider, OperationRecord, OperationStatus,
    SnapshotSelector, StoreOperationProvider,
};
use scriberig_core::{RevisionId, RootName, SnapshotId};
use scriberig_store::RootStore;
use scriberig_store_memory::MemoryStore;
use sr_change::{Migration, Provenance, SourceKind};
use std::collections::BTreeMap;

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
                change: "empty".to_owned(),
            },
        )
        .expect("commit");
    assert_eq!(committed.snapshot, Some(snapshot));
}
