//! Replays semantic migrations onto a new immutable snapshot line.

use crate::{EntityDelta, Migration, MigrationError, Operation, Provenance};
use scriberig_core::{EntityId, SnapshotId, TypeId};
use scriberig_store::{Snapshot, SnapshotRelation, SnapshotRelationChange};
use std::collections::{BTreeMap, BTreeSet};

#[path = "replay_ops.rs"]
mod replay_ops;

/// One stable semantic relation used by generic replay.
pub type Relation = (EntityId, String, EntityId);

/// Mutable-in-construction semantic state used while replaying operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayState {
    /// Entity-to-revision snapshot state.
    pub snapshot: Snapshot,
    /// Existing relation memberships.
    pub relations: BTreeSet<Relation>,
    /// Ordered collection members by collection identity.
    pub collections: BTreeMap<EntityId, Vec<EntityId>>,
}

impl ReplayState {
    /// Creates replay state from an exact snapshot with empty relations.
    ///
    /// - `snapshot`: Exact entity snapshot to replay onto.
    ///
    /// Replay state containing the supplied snapshot.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn from_snapshot(snapshot: Snapshot) -> Self {
        let relations = snapshot
            .relations()
            .map(|relation| {
                (
                    relation.from,
                    relation.relation_type.as_str().to_owned(),
                    relation.to,
                )
            })
            .collect();
        Self {
            snapshot,
            relations,
            collections: BTreeMap::new(),
        }
    }

    /// Synchronizes replay relations into the exact immutable snapshot.
    ///
    /// The snapshot identity includes all replayed relations.
    ///
    /// # Returns
    /// Unit after the exact snapshot contains the replayed relation set.
    ///
    /// # Errors
    /// Returns an invalid-operation error when a relation name is empty or
    /// snapshot storage rejects the relation changes.
    fn sync_snapshot_relations(&mut self) -> Result<(), MigrationError> {
        let desired = self
            .relations
            .iter()
            .map(|(from, relation, to)| {
                TypeId::new(relation.clone())
                    .map(|relation_type| SnapshotRelation::new(*from, relation_type, *to))
                    .map_err(|error| MigrationError::InvalidOperation {
                        reason: format!("invalid relation type: {error}"),
                    })
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let existing = self.snapshot.relations().cloned().collect::<BTreeSet<_>>();
        let changes = existing
            .difference(&desired)
            .cloned()
            .map(SnapshotRelationChange::Remove)
            .chain(
                desired
                    .difference(&existing)
                    .cloned()
                    .map(SnapshotRelationChange::Add),
            )
            .collect::<Vec<_>>();
        self.snapshot = self.snapshot.apply_relation_changes(changes);
        Ok(())
    }
}

/// Context supplied to a domain replay strategy.
#[derive(Clone, Copy, Debug)]
pub struct ReplayContext<'a> {
    /// Original migration base state.
    pub original_base: &'a Snapshot,
    /// Original migration result state.
    pub original_result: &'a Snapshot,
    /// Current target state.
    pub target: &'a ReplayState,
    /// Migration being replayed.
    pub migration: &'a Migration,
}

/// Result returned by a domain replay strategy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainReplayOutcome {
    /// Strategy applied semantic deltas.
    Applied(Vec<EntityDelta>),
    /// Strategy identified an intentional conflict.
    Conflict(String),
    /// Strategy does not understand this operation.
    NotApplicable,
}

/// Optional domain extension for replaying opaque semantic operations.
pub trait DomainReplayStrategy {
    /// Replays one operation without mutating historical input state.
    ///
    /// - `operation`: Recorded semantic operation.
    /// - `context`: Original and current exact states.
    ///
    /// Applied deltas, a structured conflict detail, or not-applicable.
    /// # Returns
    /// The operation result.
    fn replay(&self, operation: &Operation, context: ReplayContext<'_>) -> DomainReplayOutcome;
}

/// Input snapshots and migrations for one rebase operation.
#[derive(Clone, Copy, Debug)]
pub struct RebaseInput<'a> {
    /// Original base snapshot.
    pub original_base: &'a Snapshot,
    /// Original final work snapshot.
    pub original_result: &'a Snapshot,
    /// New base snapshot to receive the replay.
    pub new_base: &'a ReplayState,
    /// Historical migrations in chronological order.
    pub migrations: &'a [Migration],
}

/// A migration created on the rebased history line.
#[derive(Clone, Debug, PartialEq)]
pub struct RebasedMigration {
    /// Original migration position.
    pub source_index: usize,
    /// New migration based on the rebased line.
    pub migration: Migration,
}

/// Result of replaying semantic work onto a new base.
#[derive(Clone, Debug, PartialEq)]
pub struct RebaseResult {
    /// Original base remains unchanged.
    pub original_base: SnapshotId,
    /// Original work remains unchanged.
    pub original_result: SnapshotId,
    /// New replay starting point.
    pub new_base: SnapshotId,
    /// New result when replay completed without conflicts.
    pub new_result: Option<SnapshotId>,
    /// New migration records, each based on the rebased line.
    pub migrations: Vec<RebasedMigration>,
    /// Conflicts stop replay at the first unresolved operation.
    pub conflicts: Vec<crate::MergeConflict>,
    /// State reached before a conflict, or the complete rebased state.
    pub state: ReplayState,
}

/// Reports invalid rebase input or migration construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RebaseError {
    /// Original snapshots do not match the requested migration sequence.
    InvalidHistory {
        /// Position of the invalid migration.
        index: usize,
        /// Snapshot expected from the preceding migration.
        expected: SnapshotId,
        /// Snapshot actually declared by the migration.
        actual: SnapshotId,
    },
    /// A migration could not be created from replayed changes.
    Migration(MigrationError),
}

impl From<MigrationError> for RebaseError {
    /// Wraps migration construction failure.
    ///
    /// - `error`: Migration construction error.
    ///
    /// Rebase error containing the migration failure.
    /// # Returns
    /// The operation result.
    fn from(error: MigrationError) -> Self {
        Self::Migration(error)
    }
}

/// Replays migrations in order and stops at the first unresolved conflict.
///
/// Domain strategies run before universal operations. Universal operations use
/// exact entity, relation, and collection preconditions; they never inspect
/// rendered text or positional byte offsets.
///
/// - `input`: Original snapshots, new base, and ordered migrations.
/// - `strategy`: Optional domain replay extension.
///
/// A new history line and its resulting state. The result is never promoted.
///
/// Returns an error when migration ancestry does not match the supplied order.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn rebase(
    input: RebaseInput<'_>,
    strategy: Option<&dyn DomainReplayStrategy>,
) -> Result<RebaseResult, RebaseError> {
    let mut current = input.new_base.clone();
    let mut expected_base = input.original_base.id();
    let mut output = RebaseResult {
        original_base: input.original_base.id(),
        original_result: input.original_result.id(),
        new_base: input.new_base.snapshot.id(),
        new_result: None,
        migrations: Vec::new(),
        conflicts: Vec::new(),
        state: current.clone(),
    };
    for (index, migration) in input.migrations.iter().enumerate() {
        if migration.base != expected_base {
            return Err(RebaseError::InvalidHistory {
                index,
                expected: expected_base,
                actual: migration.base,
            });
        }
        let rebased_base = current.snapshot.id();
        let mut changes = Vec::new();
        for operation in &migration.operations {
            if let Some(strategy) = strategy {
                let context = ReplayContext {
                    original_base: input.original_base,
                    original_result: input.original_result,
                    target: &current,
                    migration,
                };
                match strategy.replay(operation, context) {
                    DomainReplayOutcome::Applied(deltas) => {
                        for delta in deltas {
                            if let Err(conflict) = replay_ops::apply_delta(&mut current, &delta) {
                                output.conflicts.push(*conflict);
                                output.state = current;
                                return Ok(output);
                            }
                            changes.push(delta);
                        }
                        continue;
                    }
                    DomainReplayOutcome::Conflict(detail) => {
                        output
                            .conflicts
                            .push(replay_ops::domain_conflict(operation, &detail));
                        output.state = current;
                        return Ok(output);
                    }
                    DomainReplayOutcome::NotApplicable => {}
                }
            }
            match replay_ops::replay_generic(operation, migration, &current) {
                Ok(Some(delta)) => {
                    if let Err(conflict) = replay_ops::apply_delta(&mut current, &delta) {
                        output.conflicts.push(*conflict);
                        output.state = current;
                        return Ok(output);
                    }
                    changes.push(delta);
                }
                Ok(None) => {}
                Err(conflict) => {
                    output.conflicts.push(*conflict);
                    output.state = current;
                    return Ok(output);
                }
            }
            replay_ops::apply_operation_state(&mut current, operation);
            current.sync_snapshot_relations()?;
        }
        if migration.operations.is_empty() {
            for delta in &migration.deltas {
                if let Err(conflict) = replay_ops::apply_delta(&mut current, delta) {
                    output.conflicts.push(*conflict);
                    output.state = current;
                    return Ok(output);
                }
                changes.push(delta.clone());
            }
        }
        current.sync_snapshot_relations()?;
        let rebased = Migration::new(
            rebased_base,
            current.snapshot.id(),
            changes,
            migration.operations.clone(),
            rebased_provenance(&migration.provenance),
        )?;
        expected_base = migration.result;
        output.migrations.push(RebasedMigration {
            source_index: index,
            migration: rebased,
        });
        output.state = current.clone();
    }
    if expected_base != input.original_result.id() {
        return Err(RebaseError::InvalidHistory {
            index: input.migrations.len(),
            expected: input.original_result.id(),
            actual: expected_base,
        });
    }
    output.new_result = Some(current.snapshot.id());
    output.state = current;
    Ok(output)
}

/// Copies provenance and records that the migration was replayed.
///
/// - `provenance`: Original migration provenance.
///
/// Copied provenance with a rebase parent marker.
/// # Returns
/// The operation result.
fn rebased_provenance(provenance: &Provenance) -> Provenance {
    let mut result = provenance.clone();
    result.parents.push("rebase".to_owned());
    result
}
