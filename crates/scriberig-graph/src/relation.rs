//! Defines owned typed semantic relation values.

use scriberig_core::{EntityId, ObjectType};

/// A relation value owned by one semantic revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationValue {
    /// Relation type identity.
    pub relation_type: ObjectType,
    /// Semantic target identities in declared order.
    pub targets: Vec<EntityId>,
}

impl RelationValue {
    /// Creates a typed relation without applying schema cardinality rules.
    ///
    /// # Parameters
    /// - `relation_type`: Relation type identity.
    /// - `targets`: Semantic target identities.
    ///
    /// # Returns
    /// The owned relation value.
    #[must_use]
    pub fn new(relation_type: ObjectType, targets: Vec<EntityId>) -> Self {
        Self {
            relation_type,
            targets,
        }
    }
}
