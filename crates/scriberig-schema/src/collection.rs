//! Defines ordered and unordered semantic collection schemas.

use crate::relation::Cardinality;
use scriberig_core::{KnowledgeClass, ObjectType};

/// Defines collection ordering semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderSemantics {
    /// Member order is semantic.
    Ordered,
    /// Member order is not semantic.
    Unordered,
}

/// Describes one semantic collection contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionSchema {
    /// Stable collection identifier.
    pub collection_type: String,
    /// Human-readable collection name.
    pub name: String,
    /// Owner entity type.
    pub owner_type: ObjectType,
    /// Permitted member entity types.
    pub member_types: Vec<ObjectType>,
    /// Ordering semantics.
    pub order: OrderSemantics,
    /// Collection cardinality.
    pub cardinality: Cardinality,
    /// Canonical or derived ownership.
    pub knowledge_class: KnowledgeClass,
}
