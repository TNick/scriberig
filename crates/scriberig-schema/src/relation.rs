//! Defines semantic relation schemas and cardinality.

use scriberig_core::{KnowledgeClass, ObjectType};

/// Bounds the number of members in a relation or collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cardinality {
    /// Minimum member count.
    pub min: u64,
    /// Optional maximum member count.
    pub max: Option<u64>,
}

impl Cardinality {
    /// Creates validated cardinality bounds.
    ///
    /// # Parameters
    /// - `min`: Minimum count.
    /// - `max`: Optional maximum count.
    ///
    /// # Returns
    /// Validated cardinality bounds.
    ///
    /// # Errors
    /// Returns an error when maximum is below minimum.
    pub fn new(min: u64, max: Option<u64>) -> Result<Self, &'static str> {
        if max.is_some_and(|value| value < min) {
            Err("maximum must not be below minimum")
        } else {
            Ok(Self { min, max })
        }
    }
    /// Returns whether a count satisfies these bounds.
    ///
    /// # Parameters
    /// - `count`: Count to check.
    ///
    /// # Returns
    /// `true` when the count is within bounds.
    #[must_use]
    pub fn accepts(&self, count: usize) -> bool {
        count as u64 >= self.min && self.max.is_none_or(|max| count as u64 <= max)
    }
}

/// Describes one semantic relation endpoint contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationSchema {
    /// Stable relation identifier.
    pub relation_type: String,
    /// Human-readable relation name.
    pub name: String,
    /// Permitted source entity types.
    pub source_types: Vec<ObjectType>,
    /// Permitted target entity types.
    pub target_types: Vec<ObjectType>,
    /// Canonical or derived ownership.
    pub knowledge_class: KnowledgeClass,
    /// Relation cardinality.
    pub cardinality: Cardinality,
    /// Whether reverse lookup is expected to be indexed.
    pub reverse_indexed: bool,
}
