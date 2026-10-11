//! Verifies structured change queries and progressive summaries.

use scriberig_core::{EntityId, RevisionId, SnapshotId};
use scriberig_store::{Snapshot, SnapshotChange, SnapshotStore};
use scriberig_store_memory::MemoryStore;
use sr_change::{
    Branch, EntityDelta, HistoryGraph, MergeInput, Migration, MigrationReference, Operation,
    Provenance, SourceKind, branches_descending_from, changed_between, common_ancestor,
    entity_history, introduced_by, merge_snapshots, migrations_touching, page,
    workspace_migrations,
};
use std::collections::BTreeMap;

/// Creates deterministic provenance for query fixtures.
///
/// # Returns
/// Provenance containing workspace and validation references.
fn provenance() -> Provenance {
    Provenance {
        source: SourceKind::User,
        actor: None,
        workspace: Some("workspace-a".to_owned()),
        projection: None,
        validation: vec!["validation-1".to_owned()],
        parents: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

/// Creates a repeated-byte snapshot identity fixture.
///
/// # Parameters
/// - `byte`: Byte repeated across the snapshot identity.
///
/// # Returns
/// A deterministic snapshot identity.
fn snapshot(byte: u8) -> SnapshotId {
    SnapshotId::new([byte; 32])
}

/// Verifies changed-between delegates to the semantic diff and preserves handles.
#[test]
fn changed_between_returns_stable_entity_revisions() {
    let store = MemoryStore::new();
    let entity = EntityId::new([1; 16]);
    let first = RevisionId::new([2; 32]);
    let second = RevisionId::new([3; 32]);
    let base = Snapshot::empty().id();
    let left = store
        .apply_changes(base, &[SnapshotChange::Set(entity, first)])
        .expect("left");
    let right = store
        .apply_changes(left, &[SnapshotChange::Set(entity, second)])
        .expect("right");
    let diff = changed_between(&store, left, right).expect("diff");
    assert_eq!(diff.deltas[0].before, Some(first));
    assert_eq!(diff.deltas[0].after, Some(second));
}

/// Verifies history questions, topology, and workspace provenance are deterministic.
#[test]
fn history_queries_return_structured_topology() {
    let entity = EntityId::new([1; 16]);
    let first = RevisionId::new([2; 32]);
    let second = RevisionId::new([3; 32]);
    let mut graph = HistoryGraph::new();
    graph.insert(MigrationReference {
        id: "a".to_owned(),
        base: snapshot(0),
        result: snapshot(1),
        parents: Vec::new(),
        deltas: vec![EntityDelta::new(entity, None, Some(first)).expect("delta")],
        workspace: Some("workspace-a".to_owned()),
    });
    graph.insert(MigrationReference {
        id: "b".to_owned(),
        base: snapshot(1),
        result: snapshot(2),
        parents: vec!["a".to_owned()],
        deltas: vec![EntityDelta::new(entity, Some(first), Some(second)).expect("delta")],
        workspace: Some("workspace-b".to_owned()),
    });
    graph.insert(MigrationReference {
        id: "c".to_owned(),
        base: snapshot(1),
        result: snapshot(3),
        parents: vec!["a".to_owned()],
        deltas: Vec::new(),
        workspace: Some("workspace-a".to_owned()),
    });
    assert_eq!(introduced_by(&graph, second)[0].id, "b");
    assert_eq!(entity_history(&graph, entity).len(), 2);
    assert_eq!(migrations_touching(&graph, entity).len(), 2);
    assert_eq!(workspace_migrations(&graph, "workspace-a").len(), 2);
    assert_eq!(
        branches_descending_from(&graph, snapshot(1)),
        vec![snapshot(2), snapshot(3)]
    );
    assert_eq!(
        common_ancestor(&graph, snapshot(2), snapshot(3)),
        Some(snapshot(1))
    );
}

/// Verifies migration and branch summaries preserve structural metadata.
#[test]
fn summaries_preserve_counts_and_optional_descriptions() {
    let entity = EntityId::new([1; 16]);
    let revision = RevisionId::new([2; 32]);
    let migration = Migration::new(
        snapshot(1),
        snapshot(2),
        vec![EntityDelta::new(entity, None, Some(revision)).expect("delta")],
        vec![Operation::CreateEntity { entity, revision }],
        provenance(),
    )
    .expect("migration");
    let summary = migration.summary();
    assert_eq!(summary.introduced, 1);
    assert_eq!(summary.operation_count, 1);
    assert_eq!(summary.validation_count, 1);
    let branch = Branch {
        id: "branch".to_owned(),
        base: snapshot(1),
        current: snapshot(2),
        title: Some("Title".to_owned()),
        purpose: None,
        description: None,
        outcome: None,
        status: Some("active".to_owned()),
        resulting_migration: Some("migration".to_owned()),
    };
    let branch_summary = branch.summary();
    assert_eq!(branch_summary.description, None);
    assert_eq!(
        branch_summary.resulting_migration.as_deref(),
        Some("migration")
    );
}

/// Verifies merge summaries count conflicts by stable kind.
#[test]
fn merge_summary_reports_conflict_kinds() {
    let entity = EntityId::new([1; 16]);
    let base = Snapshot::empty().with_revision(entity, RevisionId::new([2; 32]));
    let left = base.with_revision(entity, RevisionId::new([3; 32]));
    let right = base.with_revision(entity, RevisionId::new([4; 32]));
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
    let summary = result.summary();
    assert_eq!(summary.resolved_count, 0);
    assert_eq!(summary.conflict_count, 1);
    assert_eq!(summary.conflicts_by_kind[0].0, "BothModified");
}

/// Verifies bounded pages expose a cursor without changing item order.
#[test]
fn query_pages_are_bounded() {
    let page = page(&[1, 2, 3, 4], 1, 2);
    assert_eq!(page.items, vec![2, 3]);
    assert_eq!(page.next_cursor, Some(3));
}
