//! Verifies DXF semantic import retains source evidence and native handles.

use scriberig_core::EntityId;
use scriberig_domain_cad::CadEntity;
use scriberig_domain_dxf::{DxfImport, DxfPreviousEntity, import_dxf, import_dxf_reconciled};

/// Creates a deterministic minimal DXF with one line entity.
///
/// # Returns
/// Serialized DXF bytes containing deterministic test entities.
fn line_dxf() -> Vec<u8> {
    let mut source = dxf::Drawing::new();
    source.header.version = dxf::enums::AcadVersion::R14;
    source.add_entity(dxf::entities::Entity::new(dxf::entities::EntityType::Line(
        dxf::entities::Line {
            p1: dxf::Point::new(1.0, 2.0, 0.0),
            p2: dxf::Point::new(5.0, 6.0, 0.0),
            ..Default::default()
        },
    )));
    source.add_entity(dxf::entities::Entity::new(
        dxf::entities::EntityType::LwPolyline(dxf::entities::LwPolyline {
            vertices: vec![
                dxf::LwPolylineVertex {
                    x: 0.0,
                    y: 0.0,
                    ..Default::default()
                },
                dxf::LwPolylineVertex {
                    x: 3.0,
                    y: 4.0,
                    ..Default::default()
                },
            ],
            ..Default::default()
        }),
    ));
    let mut bytes = Vec::new();
    source.save(&mut bytes).expect("write DXF fixture");
    bytes
}

/// Imports supported entities without equating DXF handles and semantic IDs.
#[test]
fn imports_line_and_retains_exact_source_digest() {
    let bytes = line_dxf();
    let drawing_id = EntityId::new();
    let imported = import_dxf(&bytes, drawing_id).expect("read generated DXF");
    assert_eq!(imported.drawing().id, drawing_id);
    assert_eq!(imported.source_digest, *blake3::hash(&bytes).as_bytes());
    assert!(imported.complete);
    assert_eq!(imported.drawing().entities.len(), 2);
    let vertices = imported
        .drawing()
        .entities
        .iter()
        .find_map(|entity| match &entity.geometry {
            CadEntity::Polyline {
                vertices,
                closed: false,
            } => Some(vertices),
            _ => None,
        })
        .expect("polyline geometry");
    assert_eq!(vertices.len(), 2);
    assert_eq!(vertices[1].x, "3");
    assert_eq!(vertices[1].y, "4");
    assert!(
        imported
            .provider_import
            .entity_identifiers()
            .iter()
            .any(|identifier| identifier.entity == imported.drawing().entities[0].id)
    );
    assert_ne!(
        imported.drawing().entities[0].id.to_string(),
        imported.provider_import.entity_identifiers()[0]
            .value
            .as_str()
    );
}

/// Uses native handles and structural semantics only as reconciliation evidence.
#[test]
fn imported_semantic_ids_are_reconciled_separately_from_native_handles() {
    let bytes = line_dxf();
    let drawing_id = EntityId::new();
    let initial = import_dxf(&bytes, drawing_id).expect("initial import");
    let mut previous = previous_entities(&initial);
    let line = previous
        .iter_mut()
        .find(|entity| matches!(entity.semantic.geometry, CadEntity::Line { .. }))
        .expect("line entity");
    let prior_semantic_id = EntityId::new();
    let source_handle = line.native_handle.clone().expect("native handle");
    line.semantic.id = prior_semantic_id;

    let by_handle =
        import_dxf_reconciled(&bytes, drawing_id, &previous).expect("handle reconciliation");
    let resolved = by_handle
        .drawing()
        .entities
        .iter()
        .find(|entity| entity.id == prior_semantic_id)
        .expect("reconciled line");
    assert_eq!(resolved.id, prior_semantic_id);
    assert_ne!(resolved.id.to_string(), source_handle);

    let mut changed_handle_evidence = previous;
    let line = changed_handle_evidence
        .iter_mut()
        .find(|entity| entity.semantic.id == prior_semantic_id)
        .expect("prior line");
    line.native_handle = Some("changed-external-handle".to_owned());
    let by_structure = import_dxf_reconciled(&bytes, drawing_id, &changed_handle_evidence)
        .expect("structural reconciliation");
    let resolved = by_structure
        .drawing()
        .entities
        .iter()
        .find(|entity| matches!(entity.geometry, CadEntity::Line { .. }))
        .expect("structurally reconciled line");
    assert_eq!(resolved.id, prior_semantic_id);
}

/// Makes ambiguous identity continuity explicit instead of inventing a match.
#[test]
fn ambiguous_native_handle_reconciliation_downgrades_editability() {
    let bytes = line_dxf();
    let drawing_id = EntityId::new();
    let initial = import_dxf(&bytes, drawing_id).expect("initial import");
    let line = initial
        .drawing()
        .entities
        .iter()
        .find(|entity| matches!(entity.geometry, CadEntity::Line { .. }))
        .expect("line entity");
    let mut previous = previous_entities(&initial);
    let mut duplicate = previous
        .iter()
        .find(|entity| entity.semantic.id == line.id)
        .expect("line provenance")
        .clone();
    duplicate.semantic.id = EntityId::new();
    previous.push(duplicate);

    let reconciled = import_dxf_reconciled(&bytes, drawing_id, &previous)
        .expect("ambiguous source still imports for inspection");

    assert!(!reconciled.complete);
    assert!(
        reconciled
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.contains("ambiguous DXF identity reconciliation") })
    );
}

/// Builds reconciliation inputs from the semantic drawing and separate provider IDs.
///
/// # Parameters
/// - `imported`: Previous DXF import result.
///
/// # Returns
/// Prior semantic entities paired with their exact detached handle evidence.
fn previous_entities(imported: &DxfImport) -> Vec<DxfPreviousEntity> {
    imported
        .drawing()
        .entities
        .iter()
        .map(|semantic| DxfPreviousEntity {
            semantic: semantic.clone(),
            native_handle: imported
                .provider_import
                .entity_identifiers()
                .iter()
                .find(|identifier| identifier.entity == semantic.id)
                .map(|identifier| identifier.value.clone()),
        })
        .collect()
}
