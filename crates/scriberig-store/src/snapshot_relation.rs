//! Defines typed relation state carried by immutable snapshots.

use scriberig_core::{EntityId, TypeId};

/// A directed semantic relation between two entities.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SnapshotRelation {
    /// Entity at the source end of the relation.
    pub from: EntityId,
    /// Stable semantic relation type.
    pub relation_type: TypeId,
    /// Entity at the target end of the relation.
    pub to: EntityId,
}

impl SnapshotRelation {
    /// Creates a typed directed relation.
    ///
    /// - `from`: Source entity.
    /// - `relation_type`: Stable relation type.
    /// - `to`: Target entity.
    ///
    /// # Returns
    /// The relation value.
    #[must_use]
    pub fn new(from: EntityId, relation_type: TypeId, to: EntityId) -> Self {
        Self {
            from,
            relation_type,
            to,
        }
    }
}

/// Adds or removes one relation from an immutable snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotRelationChange {
    /// Adds the relation when it is not already present.
    Add(SnapshotRelation),
    /// Removes the relation when it is present.
    Remove(SnapshotRelation),
}
