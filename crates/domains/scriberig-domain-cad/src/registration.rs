//! Registers stable entity and relation names for semantic CAD drawings.

use scriberig_core::ObjectType;
use scriberig_schema::{DomainDescriptor, DomainId, SchemaVersion};

/// Addressable drawing entities represented by the CAD domain.
const ENTITY_TYPES: &[&str] = &[
    "cad.drawing",
    "cad.layer",
    "cad.entity",
    "cad.block_definition",
    "cad.layout",
    "cad.viewport",
    "cad.external_reference",
];

/// Ordered or ownership relationships represented by the CAD domain.
const RELATION_TYPES: &[&str] = &[
    "cad.layers",
    "cad.entities",
    "cad.blocks",
    "cad.layouts",
    "cad.block_entities",
    "cad.layout_viewports",
    "cad.layer_membership",
    "cad.external_references",
];

/// Returns the versioned schema descriptor for format-independent CAD semantics.
///
/// # Returns
/// Stable descriptor consumable by shared schema catalogs.
///
/// # Panics
/// Panics only if a static schema identifier is invalid.
#[must_use]
pub fn cad_schema() -> DomainDescriptor {
    DomainDescriptor {
        id: DomainId::new("cad").expect("static CAD domain id"),
        name: "Computer-aided design".to_owned(),
        schema_version: SchemaVersion::new(1),
        entity_types: ENTITY_TYPES
            .iter()
            .map(|name| ObjectType::new(*name).expect("static CAD entity type"))
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
    //! Verifies CAD semantic kinds are registered in one stable schema version.

    use super::cad_schema;

    /// Checks schema identity, version, and imported drawing kinds.
    #[test]
    fn cad_schema_registers_drawing_layers_entities_and_relations() {
        let descriptor = cad_schema();
        assert_eq!(descriptor, cad_schema());
        assert_eq!(descriptor.id.as_str(), "cad");
        assert_eq!(descriptor.schema_version.value(), 1);
        assert!(
            descriptor
                .entity_types
                .iter()
                .any(|kind| kind.as_str() == "cad.drawing")
        );
        assert!(
            descriptor
                .entity_types
                .iter()
                .any(|kind| kind.as_str() == "cad.entity")
        );
        assert!(
            descriptor
                .relation_types
                .iter()
                .any(|kind| kind == "cad.layers")
        );
    }
}
