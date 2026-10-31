//! Verifies terminal CAD summaries expose typed properties without unbounded geometry dumps.

use super::*;

/// Renders layer, entity, text, fidelity, and pagination details from one bounded page.
#[test]
fn cad_view_renders_semantic_layers_and_bounded_entity_properties() {
    let drawing = LoadState::Ready(CadDrawingInspectResponse {
        project: "project-1".to_owned(),
        snapshot: "snapshot-1".to_owned(),
        drawing: "drawing-1".to_owned(),
        units: "meters".to_owned(),
        coordinate_reference: None,
        layers: vec![CadLayerSummary {
            id: "layer-1".to_owned(),
            name: "LABELS".to_owned(),
            visible: true,
            locked: false,
        }],
        entity_count: 250,
        complete: true,
        diagnostics: Vec::new(),
    });
    let entities = LoadState::Ready(CadEntitiesResponse {
        project: "project-1".to_owned(),
        snapshot: "snapshot-1".to_owned(),
        drawing: "drawing-1".to_owned(),
        entities: vec![CadEntitySummary {
            id: "text-1".to_owned(),
            layer: "layer-1".to_owned(),
            space: CadDrawingSpace::Model,
            geometry: CadGeometry::Text {
                position: CadCoordinate {
                    x: "1".to_owned(),
                    y: "2".to_owned(),
                    z: "0".to_owned(),
                },
                value: "Parcel 124".to_owned(),
                height: "0.25".to_owned(),
            },
            style: None,
        }],
        total: 250,
        has_more: true,
    });
    let output = render(&drawing, &entities, &LoadState::NotRequested, 100);
    assert!(output.contains("LABELS"));
    assert!(output.contains("Text \"Parcel 124\""));
    assert!(output.contains("layer `layer-1`"));
    assert!(output.contains("Entities 101–101 of 250"));
    assert!(output.contains("`/cad <next-offset>`"));
    assert!(!output.contains("`AB`"));
}

/// Verifies embedded application models normalize to the same parser-free CAD DTOs.
#[test]
fn local_cad_projection_preserves_semantic_text_and_layer_identity() {
    let project = scriberig_core::ProjectId::from_bytes([161; 16]);
    let snapshot = scriberig_core::SnapshotId::new([162; 32]);
    let drawing_id = scriberig_core::EntityId::from_bytes([163; 16]);
    let layer_id = scriberig_core::EntityId::from_bytes([164; 16]);
    let entity_id = scriberig_core::EntityId::from_bytes([165; 16]);
    let drawing = drawing_from_application(scriberig_project::CadDrawingView {
        project,
        snapshot,
        drawing: drawing_id,
        units: "meters".to_owned(),
        coordinate_reference: None,
        layers: vec![scriberig_domain_cad::Layer {
            id: layer_id,
            name: "LABELS".to_owned(),
            visible: true,
            locked: false,
        }],
        entity_count: 1,
        complete: true,
        diagnostics: Vec::new(),
    });
    let page = entities_from_application(scriberig_project::CadEntityPage {
        project,
        snapshot,
        drawing: drawing_id,
        entities: vec![scriberig_domain_cad::DrawingEntity {
            id: entity_id,
            layer: layer_id,
            space: scriberig_domain_cad::DrawingSpace::Model,
            geometry: scriberig_domain_cad::CadEntity::Text {
                position: scriberig_domain_cad::Coordinate::new("1", "2", "0"),
                value: "Parcel 124".to_owned(),
                height: "0.25".to_owned(),
            },
            style: None,
        }],
        total: 1,
        has_more: false,
    });
    assert_eq!(drawing.layers[0].id, layer_id.to_string());
    assert_eq!(page.entities[0].id, entity_id.to_string());
    assert!(matches!(
        &page.entities[0].geometry,
        CadGeometry::Text { value, .. } if value == "Parcel 124"
    ));
}
