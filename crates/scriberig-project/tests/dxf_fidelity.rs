//! Verifies shared CAD inspection and exact-source DXF fidelity gates.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_domain_cad::Coordinate;
use scriberig_project::ProjectService;
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

#[path = "dxf_fidelity/opaque_candidates.rs"]
mod opaque_candidates;

/// Creates a unique project containing two deterministic DXF lines.
///
/// # Returns
/// Temporary project directory containing the generated drawing.
fn dxf_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-dxf-onboarding-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut drawing = dxf::Drawing::new();
    drawing.add_layer(dxf::tables::Layer {
        name: "PARCELS".to_owned(),
        ..Default::default()
    });
    drawing.add_layer(dxf::tables::Layer {
        name: "LABELS".to_owned(),
        ..Default::default()
    });
    let mut first =
        dxf::entities::Entity::new(dxf::entities::EntityType::Line(dxf::entities::Line {
            p1: dxf::Point::new(0.0, 0.0, 0.0),
            p2: dxf::Point::new(10.0, 0.0, 0.0),
            ..Default::default()
        }));
    "PARCELS".clone_into(&mut first.common.layer);
    drawing.add_entity(first);
    let mut second =
        dxf::entities::Entity::new(dxf::entities::EntityType::Line(dxf::entities::Line {
            p1: dxf::Point::new(20.0, 0.0, 0.0),
            p2: dxf::Point::new(30.0, 0.0, 0.0),
            ..Default::default()
        }));
    "PARCELS".clone_into(&mut second.common.layer);
    drawing.add_entity(second);
    let mut bytes = Vec::new();
    drawing.save(&mut bytes).expect("serialize DXF");
    fs::write(root.join("roads.dxf"), bytes).expect("persist DXF");
    root
}

/// Adds an unsupported DXF entity beside supported lines in a fresh project.
///
/// # Returns
/// Temporary project directory containing a mixed-support drawing.
fn dxf_project_with_unsupported_entity() -> PathBuf {
    let root = dxf_project();
    let path = root.join("roads.dxf");
    let mut drawing = dxf::Drawing::load_file(&path).expect("load drawing");
    drawing.add_block(dxf::Block {
        name: "UNMODELED".to_owned(),
        ..Default::default()
    });
    drawing.add_entity(dxf::entities::Entity::new(
        dxf::entities::EntityType::ModelPoint(dxf::entities::ModelPoint::new(dxf::Point::new(
            1.0, 2.0, 0.0,
        ))),
    ));
    drawing.save_file(&path).expect("persist mixed drawing");
    root
}

/// Creates a drawing with no supported semantic entity to edit.
///
/// # Returns
/// Temporary project directory containing one unsupported DXF point.
fn unsupported_only_dxf_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-dxf-unsupported-only-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut drawing = dxf::Drawing::new();
    drawing.add_entity(dxf::entities::Entity::new(
        dxf::entities::EntityType::ModelPoint(dxf::entities::ModelPoint::new(dxf::Point::new(
            1.0, 2.0, 0.0,
        ))),
    ));
    drawing
        .save_file(root.join("unsupported.dxf"))
        .expect("persist unsupported DXF");
    root
}

/// Adds a parcel annotation to a deterministic DXF project.
///
/// # Returns
/// Temporary project directory containing text and line entities.
fn dxf_project_with_parcel_text() -> PathBuf {
    let root = dxf_project();
    let path = root.join("roads.dxf");
    let mut drawing = dxf::Drawing::load_file(&path).expect("load drawing");
    let mut annotation =
        dxf::entities::Entity::new(dxf::entities::EntityType::Text(dxf::entities::Text {
            value: "Parcel 124".to_owned(),
            location: dxf::Point::new(2.0, 3.0, 0.0),
            text_height: 1.0,
            ..Default::default()
        }));
    "LABELS".clone_into(&mut annotation.common.layer);
    drawing.add_entity(annotation);
    let mut second_annotation =
        dxf::entities::Entity::new(dxf::entities::EntityType::Text(dxf::entities::Text {
            value: "Parcel 125".to_owned(),
            location: dxf::Point::new(4.0, 5.0, 0.0),
            text_height: 1.0,
            ..Default::default()
        }));
    "LABELS".clone_into(&mut second_annotation.common.layer);
    drawing.add_entity(second_annotation);
    drawing.save_file(&path).expect("persist annotated drawing");
    root
}

/// Scans and imports supported DXF content through the persisted project scan.
#[test]
fn dxf_file_creates_separate_semantic_drawing_layer_and_entity_records() {
    let root = dxf_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    assert_eq!(scan.files[0].adapter, "dxf");
    let project = scan.project.id;
    service.import(project).expect("import DXF");
    let manifest = service
        .artifacts(project)
        .expect("artifact fidelity")
        .into_iter()
        .find(|artifact| artifact.source.adapter == "dxf")
        .expect("DXF artifact");
    assert_eq!(
        manifest.fidelity.capability,
        scriberig_project::ArtifactCapability::LosslessEditable
    );
    assert_eq!(
        manifest.members[0].digest,
        *blake3::hash(&fs::read(root.join("roads.dxf")).expect("source bytes")).as_bytes()
    );
    let entities = store.project_entities(project).expect("semantic entities");
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.kind == "cad.drawing")
            .count(),
        1
    );
    let snapshot = service.workspace(project).expect("workspace").snapshot;
    let drawing = entities
        .iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("drawing identity");
    let view = service
        .cad_drawing(project, snapshot, drawing.entity)
        .expect("exact-snapshot CAD summary");
    assert_eq!(view.drawing, drawing.entity);
    assert_eq!(view.layers.len(), 3);
    assert!(view.layers.iter().any(|layer| layer.name == "PARCELS"));
    assert!(view.layers.iter().any(|layer| layer.name == "LABELS"));
    assert_eq!(view.entity_count, 2);
    assert!(view.complete);
    let page = service
        .cad_entities(project, snapshot, drawing.entity, 0, 1)
        .expect("bounded CAD entities");
    assert_eq!(page.entities.len(), 1);
    assert_eq!(page.total, 2);
    assert!(page.has_more);
    assert!(
        service
            .cad_entities(project, snapshot, drawing.entity, 0, 501)
            .is_err()
    );
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.kind == "cad.layer")
            .count(),
        3
    );
    assert_eq!(
        entities
            .iter()
            .filter(|entity| entity.kind == "cad.entity")
            .count(),
        2
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Filters CAD entities by semantic layer and resolves native handles.
#[test]
fn cad_queries_resolve_layer_members_and_semantic_handles() {
    let root = dxf_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    service.import(project).expect("import DXF");
    let snapshot = service.workspace(project).expect("workspace").snapshot;
    let drawing = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("drawing")
        .entity;
    let view = service
        .cad_drawing(project, snapshot, drawing)
        .expect("drawing view");
    let parcels = view
        .layers
        .iter()
        .find(|layer| layer.name == "PARCELS")
        .expect("parcels layer");
    let page = service
        .cad_entities_by_layer(project, snapshot, drawing, parcels.id, 0, 1)
        .expect("bounded entities for PARCELS");
    let lines = service
        .cad_entities_by_kind(project, snapshot, drawing, "line", 0, 10)
        .expect("line entity query");

    assert_eq!(page.total, 2);
    assert!(page.has_more);
    assert!(
        page.entities
            .iter()
            .all(|entity| entity.layer == parcels.id)
    );
    assert_eq!(lines.total, 2);
    assert!(lines.entities.iter().all(|entity| matches!(
        entity.geometry,
        scriberig_domain_cad::CadEntity::Line { .. }
    )));
    assert!(
        service
            .cad_entities_by_kind(project, snapshot, drawing, "unknown", 0, 1)
            .is_err()
    );
    let source = fs::read(root.join("roads.dxf")).expect("source DXF");
    let provider_import =
        scriberig_domain_dxf::import_dxf(&source, drawing).expect("source provider import");
    let handle = &provider_import
        .provider_import
        .entity_identifier(page.entities[0].id)
        .expect("adapter supplied external handle")
        .value;
    let resolved = service
        .cad_entity_by_native_handle(project, snapshot, drawing, handle)
        .expect("native handle resolves to semantic entity");
    assert_eq!(resolved.id, page.entities[0].id);
    assert_ne!(resolved.id.to_string(), handle.as_str());
    assert!(
        service
            .cad_entity_by_native_handle(project, snapshot, drawing, "missing")
            .is_err()
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Finds CAD text by a bounded semantic query at the exact imported snapshot.
#[test]
fn cad_text_search_finds_parcel_annotation_case_insensitively() {
    let root = dxf_project_with_parcel_text();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan project").project.id;
    service.import(project).expect("import DXF");
    let snapshot = service.workspace(project).expect("workspace").snapshot;
    let drawing = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("drawing")
        .entity;

    let page = service
        .find_cad_text(project, snapshot, drawing, "parcel 124", 0, 1)
        .expect("find parcel annotation");
    let view = service
        .cad_drawing(project, snapshot, drawing)
        .expect("drawing view");
    let labels = view
        .layers
        .iter()
        .find(|layer| layer.name == "LABELS")
        .expect("labels layer");

    assert_eq!(page.total, 1);
    assert!(!page.has_more);
    assert_eq!(page.matches[0].text, "Parcel 124");
    assert_eq!(page.matches[0].entity.layer, labels.id);
    let first = service
        .find_cad_text(project, snapshot, drawing, "parcel", 0, 1)
        .expect("first parcel text page");
    let second = service
        .find_cad_text(project, snapshot, drawing, "parcel", 1, 1)
        .expect("second parcel text page");
    assert_eq!(first.total, 2);
    assert!(first.has_more);
    assert_eq!(first.matches[0].text, "Parcel 124");
    assert!(!second.has_more);
    assert_eq!(second.matches[0].text, "Parcel 125");
    assert!(
        service
            .find_cad_text(project, snapshot, drawing, "  ", 0, 1)
            .is_err()
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Keeps unsupported-only drawings read-only even with raw-edit support enabled.
#[test]
fn unsupported_only_dxf_remains_read_only() {
    let root = unsupported_only_dxf_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan").project.id;
    service.import(project).expect("import unsupported drawing");
    let artifact = service
        .artifacts(project)
        .expect("artifact fidelity")
        .into_iter()
        .find(|artifact| artifact.source.adapter == "dxf")
        .expect("DXF artifact");
    assert_eq!(
        artifact.fidelity.capability,
        scriberig_project::ArtifactCapability::ReadOnlyDerived
    );
    assert_eq!(
        artifact.fidelity.opaque_preservation,
        scriberig_project::OpaquePreservation::Unproven
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
