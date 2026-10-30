//! Tests provider import provenance isolation.

use scriberig_core::EntityId;

use crate::{
    CadEntity, CadProviderEntityIdentifier, CadProviderIdentity, CadProviderImport, Coordinate,
    Drawing, DrawingEntity, DrawingSpace,
};

/// Builds an empty semantic drawing for provider provenance tests.
///
/// # Returns
/// An empty drawing with no layers, entities, or compound structures.
fn empty_drawing() -> Drawing {
    Drawing {
        id: EntityId::new(),
        units: "millimeter".to_owned(),
        coordinate_reference: None,
        layers: Vec::new(),
        entities: Vec::new(),
        blocks: Vec::new(),
        layouts: Vec::new(),
        external_references: Vec::new(),
    }
}

/// Keeps provider-specific identifiers outside the shared semantic drawing.
#[test]
fn providers_keep_native_identifiers_in_import_provenance() {
    let mut drawing = empty_drawing();
    let entity = EntityId::new();
    drawing.entities.push(DrawingEntity {
        id: entity,
        layer: EntityId::new(),
        space: DrawingSpace::Model,
        geometry: CadEntity::Line {
            start: Coordinate::new("0", "0", "0"),
            end: Coordinate::new("1", "0", "0"),
        },
        style: None,
    });
    let provider_a = CadProviderImport::new(
        CadProviderIdentity {
            id: "provider.a".to_owned(),
            version: "1".to_owned(),
        },
        drawing.clone(),
        vec![CadProviderEntityIdentifier {
            entity,
            value: "handle-a".to_owned(),
        }],
    )
    .unwrap();
    let provider_b = CadProviderImport::new(
        CadProviderIdentity {
            id: "provider.b".to_owned(),
            version: "2".to_owned(),
        },
        drawing,
        vec![CadProviderEntityIdentifier {
            entity,
            value: "object-id-b".to_owned(),
        }],
    )
    .unwrap();

    assert_eq!(provider_a.drawing(), provider_b.drawing());
    assert_ne!(
        provider_a.entity_identifiers()[0].value,
        provider_b.entity_identifiers()[0].value
    );
}

/// Preserves provider-reported unmapped source objects as incomplete import evidence.
#[test]
fn provider_import_exposes_unmapped_object_count() {
    let import = CadProviderImport::new(
        CadProviderIdentity {
            id: "provider.a".to_owned(),
            version: "1".to_owned(),
        },
        empty_drawing(),
        Vec::new(),
    )
    .expect("valid import")
    .with_unmapped_object_count(3);

    assert_eq!(import.unmapped_object_count(), 3);
}

/// Keeps serialized drawing entities free of provider-native identifiers.
#[test]
fn serialized_semantic_entity_has_no_provider_identifier_field() {
    let mut drawing = empty_drawing();
    drawing.entities.push(DrawingEntity {
        id: EntityId::new(),
        layer: EntityId::new(),
        space: DrawingSpace::Model,
        geometry: CadEntity::Line {
            start: Coordinate::new("0", "0", "0"),
            end: Coordinate::new("1", "0", "0"),
        },
        style: None,
    });
    let serialized = serde_json::to_value(&drawing.entities[0]).expect("serialize entity");
    assert!(serialized.get("native_handle").is_none());
    assert!(serialized.get("provider_id").is_none());
}
