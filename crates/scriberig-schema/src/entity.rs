//! Defines entity-type and domain schema containers.

use crate::{CollectionSchema, PropertySchema, RelationSchema};
use scriberig_core::ObjectType;

/// Declares one entity type and its members.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityTypeSchema {
    /// Stable entity type identifier.
    pub entity_type: ObjectType,
    /// Human-readable name.
    pub name: String,
    /// Declared properties.
    pub properties: Vec<PropertySchema>,
    /// Declared relations.
    pub relations: Vec<RelationSchema>,
    /// Declared collections.
    pub collections: Vec<CollectionSchema>,
}

/// Top-level domain schema container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    /// Stable schema identifier.
    pub id: String,
    /// Explicit schema version.
    pub version: u32,
    /// Human-readable schema name.
    pub name: String,
    /// Declared entity types.
    pub entity_types: Vec<EntityTypeSchema>,
    /// Shared relation declarations.
    pub relations: Vec<RelationSchema>,
    /// Shared collection declarations.
    pub collections: Vec<CollectionSchema>,
}
