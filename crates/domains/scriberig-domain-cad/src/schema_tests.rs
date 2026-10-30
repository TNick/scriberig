//! Tests CAD semantic identity and drawing schema invariants.

use scriberig_core::EntityId;

use crate::{CadEntity, Coordinate, Drawing, DrawingEntity, DrawingSpace, Layer};

/// Keeps provider-native identities outside serialized CAD semantic entities.
#[test]
fn semantic_entity_serialization_contains_only_shared_cad_fields() {
    let semantic_id = EntityId::new();
    let layer_id = EntityId::new();
    let entity = DrawingEntity {
        id: semantic_id,
        layer: layer_id,
        space: DrawingSpace::Model,
        geometry: CadEntity::Line {
            start: Coordinate::new("0", "0", "0"),
            end: Coordinate::new("10.25", "0", "0"),
        },
        style: None,
    };
    let drawing = Drawing {
        id: EntityId::new(),
        units: "millimeter".to_owned(),
        coordinate_reference: None,
        layers: vec![Layer {
            id: layer_id,
            name: "PARCELS".to_owned(),
            visible: true,
            locked: false,
        }],
        entities: vec![entity.clone()],
        blocks: Vec::new(),
        layouts: Vec::new(),
        external_references: Vec::new(),
    };
    assert_eq!(drawing.entities[0].id, semantic_id);
    let serialized = serde_json::to_value(&drawing.entities[0]).expect("serialize entity");
    assert!(serialized.get("native_handle").is_none());
    assert_eq!(
        serialized["id"],
        serde_json::to_value(semantic_id).expect("serialize semantic identity")
    );
}

/// Constructs a drawing with multiple layers and independently typed entities.
#[test]
fn drawing_keeps_two_layers_and_supported_entities_distinct() {
    let parcels = Layer {
        id: EntityId::new(),
        name: "PARCELS".to_owned(),
        visible: true,
        locked: false,
    };
    let labels = Layer {
        id: EntityId::new(),
        name: "LABELS".to_owned(),
        visible: true,
        locked: false,
    };
    let line = DrawingEntity {
        id: EntityId::new(),
        layer: parcels.id,
        space: DrawingSpace::Model,
        geometry: CadEntity::Line {
            start: Coordinate::new("0", "0", "0"),
            end: Coordinate::new("10", "0", "0"),
        },
        style: None,
    };
    let text = DrawingEntity {
        id: EntityId::new(),
        layer: labels.id,
        space: DrawingSpace::Model,
        geometry: CadEntity::Text {
            position: Coordinate::new("2", "3", "0"),
            value: "Parcel 124".to_owned(),
            height: "2.5".to_owned(),
        },
        style: None,
    };
    let drawing = Drawing {
        id: EntityId::new(),
        units: "millimeter".to_owned(),
        coordinate_reference: None,
        layers: vec![parcels.clone(), labels.clone()],
        entities: vec![line.clone(), text.clone()],
        blocks: Vec::new(),
        layouts: Vec::new(),
        external_references: Vec::new(),
    };

    assert_eq!(drawing.layers.len(), 2);
    assert_eq!(drawing.entities.len(), 2);
    assert_eq!(drawing.entities[0].layer, parcels.id);
    assert_eq!(drawing.entities[1].layer, labels.id);
    assert!(matches!(
        drawing.entities[0].geometry,
        CadEntity::Line { .. }
    ));
    assert!(matches!(
        drawing.entities[1].geometry,
        CadEntity::Text { .. }
    ));
}
