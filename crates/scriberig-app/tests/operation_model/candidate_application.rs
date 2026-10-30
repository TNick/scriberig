//! Verifies candidate application, relation persistence, and branch semantics.

use super::*;

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

/// Verifies semantic relation operations survive apply and commit as snapshot state.
#[test]
fn storage_apply_and_commit_persist_relation_operations() {
    let store = MemoryStore::new();
    let base = store.empty_snapshot();
    let from = EntityId::from_bytes([41; 16]);
    let to = EntityId::from_bytes([42; 16]);
    let relation = SnapshotRelation::new(
        from,
        ObjectType::new("document.references").expect("relation type"),
        to,
    );
    let expected = scriberig_store::Snapshot::empty()
        .apply_relation_changes([SnapshotRelationChange::Add(relation.clone())])
        .id();
    let migration = Migration::new(
        base,
        expected,
        Vec::new(),
        vec![sr_change::Operation::AddRelation {
            from,
            relation: "document.references".to_owned(),
            to,
        }],
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
    .expect("relation migration");
    let provider = StoreOperationProvider::new(store);
    assert!(provider.register_migration("relation-change", migration));

    let applied = provider
        .apply(
            &ApplicationContext::empty(),
            &scriberig_app::ApplyRequest {
                base_snapshot: base,
                branch: None,
                change: "relation-change".to_owned(),
            },
        )
        .expect("apply relation");
    assert_eq!(applied.snapshot, Some(expected));
    assert_eq!(
        provider.store().relations(expected),
        Ok(vec![relation.clone()])
    );

    provider
        .store()
        .set_root(RootName::new("main").expect("root"), base)
        .expect("seed root");
    let committed = provider
        .commit(
            &ApplicationContext::empty(),
            &scriberig_app::CommitRequest {
                base_snapshot: base,
                branch: None,
                change: "relation-change".to_owned(),
            },
        )
        .expect("commit relation");
    assert_eq!(committed.snapshot, Some(expected));
    assert_eq!(provider.store().relations(expected), Ok(vec![relation]));
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
