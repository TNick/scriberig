//! Verifies safe exact-target DXF line edits and rejection cases.

use scriberig_domain_cad::{Coordinate, DrawingEntity};
use scriberig_domain_dxf::{
    DxfImport, change_entity_layer, import_dxf, replace_line_geometry, replace_text,
};

/// Creates a deterministic drawing containing one semantic line.
///
/// # Returns
/// Serialized DXF bytes with two exact line entities.
fn line_dxf() -> Vec<u8> {
    let mut drawing = dxf::Drawing::new();
    drawing.header.version = dxf::enums::AcadVersion::R2000;
    drawing.header.default_drawing_units = dxf::enums::Units::Millimeters;
    drawing.add_entity(dxf::entities::Entity::new(dxf::entities::EntityType::Line(
        dxf::entities::Line {
            p1: dxf::Point::new(1.0, 2.0, 0.0),
            p2: dxf::Point::new(5.0, 6.0, 0.0),
            ..Default::default()
        },
    )));
    drawing.add_entity(dxf::entities::Entity::new(dxf::entities::EntityType::Line(
        dxf::entities::Line {
            p1: dxf::Point::new(7.0, 8.0, 0.0),
            p2: dxf::Point::new(9.0, 10.0, 0.0),
            ..Default::default()
        },
    )));
    let mut bytes = Vec::new();
    drawing.save(&mut bytes).expect("serialize source DXF");
    bytes
}

/// Replaces one LINE and proves it survives render and semantic re-import.
#[test]
fn line_geometry_edit_reopens_with_same_native_handle() {
    let source = line_dxf();
    let drawing_id = scriberig_core::EntityId::new();
    let imported = import_dxf(&source, drawing_id).expect("import source");
    let entity = &imported.drawing().entities[0];
    let handle = external_handle(&imported, entity);
    let start = Coordinate::new("10.25", "20", "0");
    let end = Coordinate::new("30", "40.5", "0");
    let output = replace_line_geometry(&source, handle, &start, &end).expect("edit line");
    let reopened = import_dxf(&output, drawing_id).expect("re-import output");
    assert_eq!(reopened.drawing().units, "millimeters");
    let entity = entity_for_handle(&reopened, handle).expect("same native entity");
    assert_eq!(
        entity.geometry,
        scriberig_domain_cad::CadEntity::Line { start, end }
    );
    assert_eq!(reopened.drawing().entities.len(), 2);
    assert!(reopened.drawing().entities.iter().any(|candidate| {
        candidate.id != entity.id
            && candidate.geometry
                == scriberig_domain_cad::CadEntity::Line {
                    start: Coordinate::new("7", "8", "0"),
                    end: Coordinate::new("9", "10", "0"),
                }
    }));
}

/// Moves one selected entity and preserves the handle and neighboring entity.
#[test]
fn layer_change_reopens_with_same_handle_and_untouched_neighbor() {
    let mut drawing =
        dxf::Drawing::load(&mut std::io::Cursor::new(line_dxf())).expect("read supported fixture");
    drawing.add_layer(dxf::tables::Layer {
        name: "LABELS".to_owned(),
        ..Default::default()
    });
    let mut source = Vec::new();
    drawing
        .save(&mut source)
        .expect("serialize layered drawing");
    let drawing_id = scriberig_core::EntityId::new();
    let imported = import_dxf(&source, drawing_id).expect("import source");
    let target = &imported.drawing().entities[0];
    let neighbor = &imported.drawing().entities[1];
    let handle = external_handle(&imported, target);
    let neighbor_before = neighbor.clone();

    let output = change_entity_layer(&source, handle, "LABELS").expect("change layer");
    let reopened = import_dxf(&output, drawing_id).expect("reopen output");
    let moved = entity_for_handle(&reopened, handle).expect("same native entity");
    let untouched = reopened
        .drawing()
        .entities
        .iter()
        .find(|entity| entity.id == neighbor_before.id)
        .expect("same neighboring entity");

    assert_eq!(moved.layer, reopened.drawing().layers[1].id);
    assert_eq!(untouched, &neighbor_before);
}

/// Rejects a layer change when the requested destination is absent.
#[test]
fn layer_change_rejects_missing_destination() {
    let source = line_dxf();
    let imported = import_dxf(&source, scriberig_core::EntityId::new()).expect("import source");
    let handle = external_handle(&imported, &imported.drawing().entities[0]);
    let error = change_entity_layer(&source, handle, "MISSING")
        .expect_err("missing layer must be rejected");

    assert_eq!(error.code, "dxf_layer_not_found");
}

/// Rejects malformed and non-finite coordinates before rendering output.
#[test]
fn invalid_line_coordinate_is_rejected() {
    let source = line_dxf();
    let imported = import_dxf(&source, scriberig_core::EntityId::new()).expect("import source");
    let handle = external_handle(&imported, &imported.drawing().entities[0]);
    let error = replace_line_geometry(
        &source,
        handle,
        &Coordinate::new("NaN", "0", "0"),
        &Coordinate::new("1", "1", "0"),
    )
    .expect_err("non-finite coordinate must be rejected");
    assert_eq!(error.code, "dxf_invalid_coordinate");
}

/// Changes one DXF label while preserving its other semantic properties.
#[test]
fn text_edit_preserves_target_properties_and_untargeted_entities() {
    let mut drawing = dxf::Drawing::new();
    let mut target =
        dxf::entities::Entity::new(dxf::entities::EntityType::Text(dxf::entities::Text {
            location: dxf::Point::new(1.0, 2.0, 3.0),
            value: "Parcel 124".to_owned(),
            text_height: 2.5,
            ..Default::default()
        }));
    target.common.layer = "LABELS".to_owned();
    drawing.add_entity(target);
    let neighbor =
        dxf::entities::Entity::new(dxf::entities::EntityType::Text(dxf::entities::Text {
            location: dxf::Point::new(4.0, 5.0, 0.0),
            value: "Keep this label".to_owned(),
            text_height: 1.25,
            ..Default::default()
        }));
    drawing.add_entity(neighbor);
    let mut source = Vec::new();
    drawing.save(&mut source).expect("serialize text source");
    let drawing_id = scriberig_core::EntityId::new();
    let before = import_dxf(&source, drawing_id).expect("import source");
    let target = &before.drawing().entities[0];
    let handle = external_handle(&before, target);
    let mut expected_target = target.clone();
    let scriberig_domain_cad::CadEntity::Text { value, .. } = &mut expected_target.geometry else {
        panic!("expected text geometry");
    };
    *value = "Parcel 125".to_owned();

    let output = replace_text(&source, handle, "Parcel 125").expect("edit label");
    let after = import_dxf(&output, drawing_id).expect("reopen edited source");

    assert_eq!(after.drawing().entities[0], expected_target);
    assert_eq!(after.drawing().entities[1], before.drawing().entities[1]);
}

/// Edits model-space geometry while retaining unmodeled block records exactly.
#[test]
fn unmodeled_block_survives_line_edit_byte_for_byte() {
    let mut drawing =
        dxf::Drawing::load(&mut std::io::Cursor::new(line_dxf())).expect("read supported fixture");
    drawing.add_block(dxf::Block {
        name: "UNMODELED".to_owned(),
        ..Default::default()
    });
    let mut source = Vec::new();
    drawing
        .save(&mut source)
        .expect("serialize unsupported fixture");
    let imported = import_dxf(&source, scriberig_core::EntityId::new()).expect("import source");
    assert!(!imported.complete);
    assert!(imported.diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("block records are not represented semantically")
    }));
    let handle = external_handle(&imported, &imported.drawing().entities[0]);
    let output = replace_line_geometry(
        &source,
        handle,
        &Coordinate::new("1", "2", "0"),
        &Coordinate::new("3", "4", "0"),
    )
    .expect("raw edit retains unsupported source records");
    scriberig_domain_dxf::verify_opaque_preservation(
        &source,
        &output,
        handle,
        &[10, 20, 30, 11, 21, 31],
    )
    .expect("only selected line coordinates changed");
    let mut corrupted = output.clone();
    let block_name = b"UNMODELED";
    let offset = corrupted
        .windows(block_name.len())
        .position(|window| window == block_name)
        .expect("unmodeled block name remains present");
    corrupted[offset] = b'X';
    let error = scriberig_domain_dxf::verify_opaque_preservation(
        &source,
        &corrupted,
        handle,
        &[10, 20, 30, 11, 21, 31],
    )
    .expect_err("changed opaque source data must fail validation");
    assert_eq!(error.code, "dxf_opaque_preservation_failed");
    let reopened = import_dxf(&output, imported.drawing().id).expect("reopen edited source");
    assert_eq!(
        reopened.drawing().entities[0].geometry,
        scriberig_domain_cad::CadEntity::Line {
            start: Coordinate::new("1", "2", "0"),
            end: Coordinate::new("3", "4", "0"),
        }
    );
}

/// Edits text inside a supported entity while preserving an unmodeled block.
#[test]
fn unmodeled_block_survives_text_edit_byte_for_byte() {
    let mut drawing =
        dxf::Drawing::load(&mut std::io::Cursor::new(line_dxf())).expect("read supported fixture");
    drawing.add_block(dxf::Block {
        name: "UNMODELED".to_owned(),
        ..Default::default()
    });
    let text = dxf::entities::Entity::new(dxf::entities::EntityType::Text(dxf::entities::Text {
        value: "Parcel 124".to_owned(),
        ..Default::default()
    }));
    drawing.add_entity(text);
    let mut source = Vec::new();
    drawing.save(&mut source).expect("serialize mixed fixture");
    let drawing_id = scriberig_core::EntityId::new();
    let imported = import_dxf(&source, drawing_id).expect("import mixed fixture");
    let target = imported
        .drawing()
        .entities
        .iter()
        .find(|entity| {
            matches!(
                entity.geometry,
                scriberig_domain_cad::CadEntity::Text { .. }
            )
        })
        .expect("semantic text entity");
    let handle = external_handle(&imported, target);
    let output = replace_text(&source, handle, "Parcel 125").expect("edit supported text");
    scriberig_domain_dxf::verify_opaque_preservation(&source, &output, handle, &[1])
        .expect("only authored text field changed");
    let reopened = import_dxf(&output, drawing_id).expect("reopen edited source");
    assert!(reopened.drawing().entities.iter().any(|entity| {
        matches!(&entity.geometry, scriberig_domain_cad::CadEntity::Text { value, .. } if value == "Parcel 125")
    }));
}

/// Reopens an opaque-source layer edit and proves the block bytes are unchanged.
#[test]
fn unmodeled_block_survives_layer_edit_byte_for_byte() {
    let mut drawing =
        dxf::Drawing::load(&mut std::io::Cursor::new(line_dxf())).expect("read supported fixture");
    drawing.add_layer(dxf::tables::Layer {
        name: "LABELS".to_owned(),
        ..Default::default()
    });
    drawing.add_block(dxf::Block {
        name: "UNMODELED".to_owned(),
        ..Default::default()
    });
    let mut source = Vec::new();
    drawing.save(&mut source).expect("serialize mixed fixture");
    let drawing_id = scriberig_core::EntityId::new();
    let imported = import_dxf(&source, drawing_id).expect("import mixed fixture");
    let target = &imported.drawing().entities[0];
    let handle = external_handle(&imported, target);
    let output = change_entity_layer(&source, handle, "LABELS").expect("change target layer");
    scriberig_domain_dxf::verify_opaque_preservation(&source, &output, handle, &[8])
        .expect("only the selected entity layer changed");
    let reopened = import_dxf(&output, drawing_id).expect("reopen edited drawing");
    let moved = entity_for_handle(&reopened, handle).expect("same entity handle");
    let labels = reopened
        .drawing()
        .layers
        .iter()
        .find(|layer| layer.name == "LABELS")
        .expect("destination layer");
    assert_eq!(moved.layer, labels.id);
}

/// Resolves a provider-owned DXF handle for one semantic entity in an import.
///
/// # Parameters
/// - `imported`: Parsed drawing with detached provider provenance.
/// - `entity`: Semantic entity whose source handle is requested.
///
/// # Returns
/// The external handle value from the separate import provenance envelope.
fn external_handle<'a>(imported: &'a DxfImport, entity: &DrawingEntity) -> &'a str {
    imported
        .provider_import
        .entity_identifiers()
        .iter()
        .find(|identifier| identifier.entity == entity.id)
        .expect("entity external identity")
        .value
        .as_str()
}

/// Resolves one semantic entity from a handle held in provider provenance.
///
/// # Parameters
/// - `imported`: Parsed drawing with detached provider provenance.
/// - `handle`: External DXF handle to resolve.
///
/// # Returns
/// The matching semantic entity when the import recorded that handle.
fn entity_for_handle<'a>(imported: &'a DxfImport, handle: &str) -> Option<&'a DrawingEntity> {
    let identifier = imported
        .provider_import
        .entity_identifiers()
        .iter()
        .find(|identifier| identifier.value == handle)?;
    imported
        .drawing()
        .entities
        .iter()
        .find(|entity| entity.id == identifier.entity)
}
