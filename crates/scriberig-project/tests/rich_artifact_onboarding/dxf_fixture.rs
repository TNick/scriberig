//! Verifies the checked-in DXF fixture exposes the CAD golden-path anchors.

use std::fs;

/// Verifies the checked-in DXF fixture contains its layer, geometry, text, and preservation anchors.
#[test]
fn checked_in_dxf_fixture_imports_cad_conformance_anchors() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/drawing.dxf");
    let bytes = fs::read(path).expect("checked-in DXF fixture");
    let imported = scriberig_domain_dxf::import_dxf(&bytes, scriberig_core::EntityId::new())
        .expect("fixture parses as DXF");

    assert!(imported.complete, "fixture is inside the supported subset");
    assert_eq!(imported.drawing().units, "meters");
    assert_eq!(imported.drawing().layers.len(), 3);
    assert!(
        imported
            .drawing()
            .layers
            .iter()
            .any(|layer| layer.name == "PARCELS")
    );
    assert!(
        imported
            .drawing()
            .layers
            .iter()
            .any(|layer| layer.name == "LABELS")
    );
    assert!(imported.drawing().entities.iter().any(|entity| matches!(
        &entity.geometry,
        scriberig_domain_cad::CadEntity::Polyline { vertices, .. } if vertices.len() == 3
    )));
    assert!(imported.drawing().entities.iter().any(|entity| matches!(
        &entity.geometry,
        scriberig_domain_cad::CadEntity::Text { value, .. } if value == "Parcel 124"
    )));
    assert!(imported.drawing().entities.iter().any(|entity| matches!(
        &entity.geometry,
        scriberig_domain_cad::CadEntity::Circle { .. }
    )));
}
