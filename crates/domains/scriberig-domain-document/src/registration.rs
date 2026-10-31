//! Registers stable shared entity and relation names for rich documents.

use scriberig_core::ObjectType;
use scriberig_schema::{DomainDescriptor, DomainId, SchemaVersion};

/// Rich document entity kinds imported as independently addressable project records.
const ENTITY_TYPES: &[&str] = &[
    "document.root",
    "document.section",
    "document.paragraph",
    "document.table",
    "document.table_row",
    "document.table_cell",
    "document.drawing",
];

/// Ordered containment relations in generic rich document structure.
const RELATION_TYPES: &[&str] = &[
    "document.sections",
    "document.blocks",
    "document.rows",
    "document.cells",
];

/// Returns the deterministic shared descriptor for addressable rich documents.
///
/// # Returns
/// Domain descriptor consumed by schema catalogs and adapters.
///
/// # Panics
/// Panics only if one of the static identifiers is invalid.
#[must_use]
pub fn document_schema() -> DomainDescriptor {
    DomainDescriptor {
        id: DomainId::new("document").expect("static document domain id"),
        name: "Rich document".to_owned(),
        schema_version: SchemaVersion::new(1),
        entity_types: ENTITY_TYPES
            .iter()
            .map(|name| ObjectType::new(*name).expect("static document entity type"))
            .collect(),
        relation_types: RELATION_TYPES
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
        capabilities: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::document_schema;

    /// Verifies schema registration names the document entities and ordering relations.
    #[test]
    fn document_schema_is_stable_and_complete_for_imported_entities() {
        let first = document_schema();
        assert_eq!(first, document_schema());
        assert_eq!(first.entity_types.len(), 7);
        assert_eq!(first.relation_types.len(), 4);
        assert!(
            first
                .entity_types
                .iter()
                .any(|kind| kind.as_str() == "document.paragraph")
        );
    }
}
