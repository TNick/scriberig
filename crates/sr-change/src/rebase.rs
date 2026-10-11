//! Replays semantic migrations onto a new immutable snapshot line.

use crate::{EntityDelta, Migration, MigrationError, Operation, Provenance};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_store::Snapshot;
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
    /// # Parameters
    /// - `snapshot`: Exact entity snapshot to replay onto.
    ///
    /// # Returns
    /// Replay state containing the supplied snapshot.
    #[must_use]
    pub fn from_snapshot(snapshot: Snapshot) -> Self {
        Self {
            snapshot,
            relations: BTreeSet::new(),
            collections: BTreeMap::new(),
        }
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
    /// # Parameters
    /// - `operation`: Recorded semantic operation.
    /// - `context`: Original and current exact states.
    ///
    /// # Returns
    /// Applied deltas, a structured conflict detail, or not-applicable.
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
    /// # Parameters
    /// - `error`: Migration construction error.
    ///
    /// # Returns
    /// Rebase error containing the migration failure.
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
/// # Parameters
/// - `input`: Original snapshots, new base, and ordered migrations.
/// - `strategy`: Optional domain replay extension.
///
/// # Returns
/// A new history line and its resulting state. The result is never promoted.
///
/// # Errors
/// Returns an error when migration ancestry does not match the supplied order.
#[allow(clippy::too_many_lines)]
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
/// # Parameters
/// - `provenance`: Original migration provenance.
///
/// # Returns
/// Copied provenance with a rebase parent marker.
fn rebased_provenance(provenance: &Provenance) -> Provenance {
    let mut result = provenance.clone();
    result.parents.push("rebase".to_owned());
    result
}
