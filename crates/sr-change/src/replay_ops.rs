//! Implements generic semantic operation replay and precondition checks.

use super::ReplayState;
use crate::{ConflictKind, EntityDelta, Migration, Operation};
use scriberig_core::EntityId;

/// Replays one universal operation against exact semantic preconditions.
///
/// - `operation`: Recorded operation.
/// - `migration`: Original migration carrying expected revisions.
/// - `state`: Current rebased state.
///
/// An entity delta, no entity delta, or a structured conflict.
///
/// Returns a conflict when an operation precondition fails.
#[allow(clippy::too_many_lines)]
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn replay_generic(
    operation: &Operation,
    migration: &Migration,
    state: &ReplayState,
) -> Result<Option<EntityDelta>, Box<crate::MergeConflict>> {
    let delta_for = |entity| migration.deltas.iter().find(|delta| delta.entity == entity);
    match operation {
        Operation::CreateEntity { entity, revision } => {
            if state.snapshot.resolve(*entity).is_some() {
                return Err(Box::new(entity_conflict(
                    *entity,
                    ConflictKind::BothModified,
                    "create precondition failed",
                )));
            }
            Ok(Some(
                EntityDelta::new(*entity, None, Some(*revision)).expect("create delta"),
            ))
        }
        Operation::ReplaceRevision { entity, revision } => {
            let Some(delta) = delta_for(*entity) else {
                return Err(Box::new(entity_conflict(
                    *entity,
                    ConflictKind::BothModified,
                    "replace operation has no recorded precondition",
                )));
            };
            let expected = delta.before;
            if state.snapshot.resolve(*entity) != expected {
                return Err(Box::new(entity_conflict(
                    *entity,
                    ConflictKind::BothModified,
                    "replace precondition failed",
                )));
            }
            Ok(Some(
                EntityDelta::new(*entity, expected, Some(*revision)).expect("replace delta"),
            ))
        }
        Operation::RemoveEntity { entity } => {
            let Some(delta) = delta_for(*entity) else {
                return Err(Box::new(entity_conflict(
                    *entity,
                    ConflictKind::DeleteModify,
                    "remove operation has no recorded precondition",
                )));
            };
            let expected = delta.before;
            if state.snapshot.resolve(*entity) != expected {
                return Err(Box::new(entity_conflict(
                    *entity,
                    ConflictKind::DeleteModify,
                    "remove precondition failed",
                )));
            }
            Ok(Some(
                EntityDelta::new(*entity, expected, None).expect("remove delta"),
            ))
        }
        Operation::AddRelation { from, relation, to } => {
            let edge = (*from, relation.clone(), *to);
            if state.relations.contains(&edge) {
                return Err(Box::new(entity_conflict(
                    *from,
                    ConflictKind::RelationConflict,
                    "relation already exists",
                )));
            }
            Ok(None)
        }
        Operation::RemoveRelation { from, relation, to } => {
            let edge = (*from, relation.clone(), *to);
            if !state.relations.contains(&edge) {
                return Err(Box::new(entity_conflict(
                    *from,
                    ConflictKind::RelationConflict,
                    "relation is absent",
                )));
            }
            Ok(None)
        }
        Operation::InsertCollectionMember {
            collection,
            member,
            position,
        } => {
            let values = state
                .collections
                .get(collection)
                .cloned()
                .unwrap_or_default();
            if usize::try_from(position.index).map_or(true, |index| index > values.len())
                || values.contains(member)
            {
                return Err(Box::new(entity_conflict(
                    *collection,
                    ConflictKind::CollectionConflict,
                    "collection insertion precondition failed",
                )));
            }
            Ok(None)
        }
        Operation::RemoveCollectionMember { collection, member }
        | Operation::MoveCollectionMember {
            collection, member, ..
        } => {
            let values = state
                .collections
                .get(collection)
                .cloned()
                .unwrap_or_default();
            if !values.contains(member) {
                return Err(Box::new(entity_conflict(
                    *collection,
                    ConflictKind::CollectionConflict,
                    "collection member precondition failed",
                )));
            }
            Ok(None)
        }
        Operation::ReplaceCollectionStructure {
            collection,
            members,
        } => {
            let values = state
                .collections
                .get(collection)
                .cloned()
                .unwrap_or_default();
            if values == *members {
                return Err(Box::new(entity_conflict(
                    *collection,
                    ConflictKind::CollectionConflict,
                    "collection replacement is a no-op",
                )));
            }
            Ok(None)
        }
        Operation::Domain { .. } => Err(Box::new(entity_conflict(
            EntityId::from_bytes([0; 16]),
            ConflictKind::DomainConflict,
            "domain replay strategy unavailable",
        ))),
    }
}

/// Applies one entity delta after checking its expected revision.
///
/// - `state`: Mutable rebased state.
/// - `delta`: Expected entity transition.
///
/// Nothing when the delta was applied.
///
/// Returns a conflict when the current revision differs.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn apply_delta(
    state: &mut ReplayState,
    delta: &EntityDelta,
) -> Result<(), Box<crate::MergeConflict>> {
    if state.snapshot.resolve(delta.entity) != delta.before {
        return Err(Box::new(entity_conflict(
            delta.entity,
            ConflictKind::BothModified,
            "entity delta precondition failed",
        )));
    }
    state.snapshot = match delta.after {
        Some(revision) => state.snapshot.with_revision(delta.entity, revision),
        None => state.snapshot.without_entity(delta.entity),
    };
    Ok(())
}

/// Applies relation and collection state for a generic operation.
///
/// - `state`: Mutable rebased state.
/// - `operation`: Operation whose semantic state should be recorded.
pub fn apply_operation_state(state: &mut ReplayState, operation: &Operation) {
    match operation {
        Operation::AddRelation { from, relation, to } => {
            state.relations.insert((*from, relation.clone(), *to));
        }
        Operation::RemoveRelation { from, relation, to } => {
            state.relations.remove(&(*from, relation.clone(), *to));
        }
        Operation::InsertCollectionMember {
            collection,
            member,
            position,
        } => {
            let values = state.collections.entry(*collection).or_default();
            if let Ok(index) = usize::try_from(position.index) {
                values.insert(index, *member);
            }
        }
        Operation::RemoveCollectionMember { collection, member } => {
            if let Some(values) = state.collections.get_mut(collection) {
                values.retain(|value| value != member);
            }
        }
        Operation::MoveCollectionMember {
            collection,
            member,
            position,
        } => {
            if let Some(values) = state.collections.get_mut(collection) {
                values.retain(|value| value != member);
                if let Ok(index) = usize::try_from(position.index) {
                    values.insert(index, *member);
                }
            }
        }
        Operation::ReplaceCollectionStructure {
            collection,
            members,
        } => {
            state.collections.insert(*collection, members.clone());
        }
        Operation::CreateEntity { .. }
        | Operation::ReplaceRevision { .. }
        | Operation::RemoveEntity { .. }
        | Operation::Domain { .. } => {}
    }
}

/// Builds a conflict with a stable semantic entity and detail.
///
/// - `entity`: Entity associated with the conflict.
/// - `kind`: Structured conflict category.
/// - `detail`: Safe explanation of the failed precondition.
///
/// A structured semantic conflict.
/// # Returns
/// The operation result.
fn entity_conflict(entity: EntityId, kind: ConflictKind, detail: &str) -> crate::MergeConflict {
    crate::MergeConflict {
        entity,
        base: None,
        left: None,
        right: None,
        kind,
        path: None,
        detail: Some(detail.to_owned()),
    }
}

/// Builds a domain conflict associated with an operation target.
///
/// - `operation`: Operation that requires domain interpretation.
/// - `detail`: Safe strategy or precondition detail.
///
/// A structured domain conflict.
/// # Returns
/// The operation result.
pub fn domain_conflict(operation: &Operation, detail: &str) -> crate::MergeConflict {
    let entity = match operation {
        Operation::CreateEntity { entity, .. }
        | Operation::ReplaceRevision { entity, .. }
        | Operation::RemoveEntity { entity } => *entity,
        Operation::AddRelation { from, .. } | Operation::RemoveRelation { from, .. } => *from,
        Operation::InsertCollectionMember { collection, .. }
        | Operation::RemoveCollectionMember { collection, .. }
        | Operation::MoveCollectionMember { collection, .. }
        | Operation::ReplaceCollectionStructure { collection, .. } => *collection,
        Operation::Domain { .. } => EntityId::from_bytes([0; 16]),
    };
    entity_conflict(entity, ConflictKind::DomainConflict, detail)
}
