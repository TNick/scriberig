//! Covers semantic rebase and replay behavior.

use scriberig_core::{EntityId, ObjectType, RevisionId, SnapshotId};
use scriberig_store::{Snapshot, SnapshotRelation, SnapshotRelationChange};
use sr_change::{
    ConflictKind, DomainReplayOutcome, DomainReplayStrategy, EntityDelta, Migration, Operation,
    Provenance, RebaseInput, ReplayContext, ReplayState, SourceKind, rebase,
};
use std::collections::BTreeMap;

/// Creates deterministic provenance for replay fixtures.
///
/// Empty deterministic provenance.
/// # Returns
/// The operation result.
fn provenance() -> Provenance {
    Provenance {
        source: SourceKind::User,
        actor: None,
        workspace: None,
        projection: None,
        validation: Vec::new(),
        parents: Vec::new(),
        metadata: BTreeMap::new(),
    }
}

/// Creates one operation-bearing migration fixture.
///
/// - `base`: Original migration base.
/// - `result`: Original migration result.
/// - `delta`: Entity transition carried by the migration.
/// - `operation`: Recorded semantic operation.
///
/// A validated migration fixture.
/// # Returns
/// The operation result.
fn migration(
    base: SnapshotId,
    result: SnapshotId,
    delta: EntityDelta,
    operation: Operation,
) -> Migration {
    Migration::new(base, result, vec![delta], vec![operation], provenance()).expect("migration")
}

/// Verifies one semantic revision operation creates a new descendant line.
#[test]
fn rebase_replays_revision_and_preserves_original_history() {
    let entity = EntityId::from_bytes([1; 16]);
    let old = RevisionId::new([2; 32]);
    let next = RevisionId::new([3; 32]);
    let base = Snapshot::empty().with_revision(entity, old);
    let original = base.with_revision(entity, next);
    let migration = migration(
        base.id(),
        original.id(),
        sr_change::EntityDelta::new(entity, Some(old), Some(next)).expect("delta"),
        Operation::ReplaceRevision {
            entity,
            revision: next,
        },
    );
    let new_base = ReplayState::from_snapshot(base.clone());
    let result = rebase(
        RebaseInput {
            original_base: &base,
            original_result: &original,
            new_base: &new_base,
            migrations: &[migration],
        },
        None,
    )
    .expect("rebase");
    assert_eq!(result.conflicts, Vec::new());
    assert_eq!(result.state.snapshot.resolve(entity), Some(next));
    assert_eq!(result.migrations[0].migration.base, base.id());
    assert_eq!(result.original_result, original.id());
}

/// Verifies that a changed target fails the exact replace precondition.
#[test]
fn rebase_reports_conflict_for_changed_target() {
    let entity = EntityId::from_bytes([1; 16]);
    let old = RevisionId::new([2; 32]);
    let next = RevisionId::new([3; 32]);
    let other = RevisionId::new([4; 32]);
    let base = Snapshot::empty().with_revision(entity, old);
    let original = base.with_revision(entity, next);
    let migration = migration(
        base.id(),
        original.id(),
        sr_change::EntityDelta::new(entity, Some(old), Some(next)).expect("delta"),
        Operation::ReplaceRevision {
            entity,
            revision: next,
        },
    );
    let new_base = ReplayState::from_snapshot(base.with_revision(entity, other));
    let result = rebase(
        RebaseInput {
            original_base: &base,
            original_result: &original,
            new_base: &new_base,
            migrations: &[migration],
        },
        None,
    )
    .expect("rebase");
    assert_eq!(result.conflicts[0].kind, ConflictKind::BothModified);
    assert_eq!(result.state.snapshot.resolve(entity), Some(other));
}

/// Verifies relation preconditions and application use stable semantic edges.
#[test]
fn rebase_replays_relation_operation() {
    let from = EntityId::from_bytes([1; 16]);
    let to = EntityId::from_bytes([2; 16]);
    let base = Snapshot::empty();
    let original = base.apply_relation_changes([SnapshotRelationChange::Add(
        SnapshotRelation::new(from, ObjectType::new("links").expect("relation type"), to),
    )]);
    let operation = Operation::AddRelation {
        from,
        relation: "links".to_owned(),
        to,
    };
    let migration = Migration::new(
        base.id(),
        original.id(),
        Vec::new(),
        vec![operation],
        provenance(),
    )
    .expect("migration");
    let result = rebase(
        RebaseInput {
            original_base: &base,
            original_result: &original,
            new_base: &ReplayState::from_snapshot(base.clone()),
            migrations: &[migration],
        },
        None,
    )
    .expect("rebase");
    assert_eq!(result.conflicts.len(), 0);
    assert!(
        result
            .state
            .relations
            .contains(&(from, "links".to_owned(), to))
    );
    assert_eq!(result.new_result, Some(original.id()));
    assert_eq!(result.state.snapshot.id(), original.id());
}

/// Verifies domain operations use a strategy before generic replay.
#[test]
fn rebase_domain_strategy_resolves_unknown_operation() {
    /// Resolves opaque domain operations for the replay test.
    struct Strategy;
    impl DomainReplayStrategy for Strategy {
        /// Resolves the custom operation into no entity delta.
        ///
        /// - `operation`: Recorded operation.
        /// - `_context`: Original and target replay context.
        ///
        /// Applied for the custom domain operation, otherwise not applicable.
        /// # Returns
        /// The operation result.
        fn replay(
            &self,
            operation: &Operation,
            _context: ReplayContext<'_>,
        ) -> DomainReplayOutcome {
            match operation {
                Operation::Domain { .. } => DomainReplayOutcome::Applied(Vec::new()),
                _ => DomainReplayOutcome::NotApplicable,
            }
        }
    }
    let base = Snapshot::empty();
    let operation = Operation::Domain {
        domain: scriberig_core::ObjectType::new("test").expect("type"),
        operation: "custom".to_owned(),
        payload: scriberig_core::KnowledgeValue::Null,
    };
    let migration = Migration::new(
        base.id(),
        base.id(),
        Vec::new(),
        vec![operation],
        provenance(),
    )
    .expect("migration");
    let result = rebase(
        RebaseInput {
            original_base: &base,
            original_result: &base,
            new_base: &ReplayState::from_snapshot(base.clone()),
            migrations: &[migration],
        },
        Some(&Strategy),
    )
    .expect("rebase");
    assert_eq!(result.conflicts.len(), 0);
}

/// Verifies ordered replay of two migrations keeps both rebased edges.
#[test]
fn rebase_replays_multiple_migrations_in_order() {
    let entity = EntityId::from_bytes([1; 16]);
    let first = RevisionId::new([2; 32]);
    let second = RevisionId::new([3; 32]);
    let third = RevisionId::new([4; 32]);
    let base = Snapshot::empty().with_revision(entity, first);
    let middle = base.with_revision(entity, second);
    let original = middle.with_revision(entity, third);
    let first_migration = migration(
        base.id(),
        middle.id(),
        sr_change::EntityDelta::new(entity, Some(first), Some(second)).expect("delta"),
        Operation::ReplaceRevision {
            entity,
            revision: second,
        },
    );
    let second_migration = migration(
        middle.id(),
        original.id(),
        sr_change::EntityDelta::new(entity, Some(second), Some(third)).expect("delta"),
        Operation::ReplaceRevision {
            entity,
            revision: third,
        },
    );
    let result = rebase(
        RebaseInput {
            original_base: &base,
            original_result: &original,
            new_base: &ReplayState::from_snapshot(base.clone()),
            migrations: &[first_migration, second_migration],
        },
        None,
    )
    .expect("rebase");
    assert_eq!(result.conflicts.len(), 0);
    assert_eq!(result.migrations.len(), 2);
    assert_eq!(
        result.migrations[1].migration.base,
        result.migrations[0].migration.result
    );
    assert_eq!(result.state.snapshot.resolve(entity), Some(third));
}

/// Verifies unknown domain intent becomes a conflict instead of being dropped.
#[test]
fn rebase_missing_domain_strategy_is_conflict() {
    let base = Snapshot::empty();
    let operation = Operation::Domain {
        domain: ObjectType::new("test").expect("type"),
        operation: "custom".to_owned(),
        payload: scriberig_core::KnowledgeValue::Null,
    };
    let migration = Migration::new(
        base.id(),
        base.id(),
        Vec::new(),
        vec![operation],
        provenance(),
    )
    .expect("migration");
    let result = rebase(
        RebaseInput {
            original_base: &base,
            original_result: &base,
            new_base: &ReplayState::from_snapshot(base.clone()),
            migrations: &[migration],
        },
        None,
    )
    .expect("rebase");
    assert_eq!(result.conflicts[0].kind, ConflictKind::DomainConflict);
    assert_eq!(result.migrations.len(), 0);
}
