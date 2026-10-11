//! Builds the language-independent code schema descriptor.
use crate::types::{ENTITY_TYPES, RELATIONS};
use scriberig_schema::{DomainDescriptor, DomainId, SchemaVersion};
/// Returns the deterministic shared code schema.
///
/// # Returns
/// Domain descriptor.
///
/// # Panics
/// Panics only if a static identifier is invalid.
#[must_use]
pub fn code_schema() -> DomainDescriptor {
    DomainDescriptor {
        id: DomainId::new("code").expect("static domain"),
        name: "Code".to_owned(),
        schema_version: SchemaVersion::new(1),
        entity_types: ENTITY_TYPES
            .iter()
            .map(|name| scriberig_core::ObjectType::new(*name).expect("static type"))
            .collect(),
        relation_types: RELATIONS.iter().map(|name| (*name).to_owned()).collect(),
        capabilities: Vec::new(),
    }
}
