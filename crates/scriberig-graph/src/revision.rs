//! Defines storage-independent exact semantic revision records.

use crate::{CollectionValue, RelationValue};
use scriberig_core::{EntityId, KnowledgeValue, ObjectType};
use scriberig_schema::ValidationSubject;
use std::collections::BTreeMap;

/// Exact semantic state for one entity at one immutable revision.
#[derive(Clone, Debug, PartialEq)]
pub struct RevisionRecord {
    /// Stable semantic entity identity.
    pub entity: EntityId,
    /// Declared entity type.
    pub entity_type: ObjectType,
    /// Canonical scalar and structured properties.
    pub properties: BTreeMap<String, KnowledgeValue>,
    /// Owned forward relations.
    pub relations: BTreeMap<String, RelationValue>,
    /// Owned collection values.
    pub collections: BTreeMap<String, CollectionValue>,
}

impl ValidationSubject for RevisionRecord {
    /// Returns the declared entity type.
    ///
    /// The entity type identity.
    /// # Returns
    /// The operation result.
    fn entity_type(&self) -> &ObjectType {
        &self.entity_type
    }
    /// Returns canonical properties.
    ///
    /// The deterministic property map.
    /// # Returns
    /// The operation result.
    fn properties(&self) -> &BTreeMap<String, KnowledgeValue> {
        &self.properties
    }
}
