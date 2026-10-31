//! Verifies project reimports reconcile CAD identities across native handle changes.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_core::EntityId;
use scriberig_domain_cad::{CadEntity, DrawingEntity};
use scriberig_domain_dxf::DxfImport;
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Creates a temporary DXF project containing two distinguishable lines.
///
/// # Returns
/// Directory containing the persisted DXF source.
fn create_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!("scriberig-cad-reconcile-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("create project directory");
    let mut drawing = dxf::Drawing::new();
    drawing.add_layer(dxf::tables::Layer {
        name: "PARCELS".to_owned(),
        ..Default::default()
    });
    for (start, end) in [(0.0, 10.0), (20.0, 30.0)] {
        let mut entity =
            dxf::entities::Entity::new(dxf::entities::EntityType::Line(dxf::entities::Line {
                p1: dxf::Point::new(start, 0.0, 0.0),
                p2: dxf::Point::new(end, 0.0, 0.0),
                ..Default::default()
            }));
        "PARCELS".clone_into(&mut entity.common.layer);
        drawing.add_entity(entity);
    }
    drawing
        .save_file(root.join("drawing.dxf"))
        .expect("write DXF project source");
    root
}

/// Returns semantic entity IDs indexed by exact line start coordinate.
///
/// # Parameters
/// - `entities`: Snapshot-bound CAD entities to inspect.
/// - `imported`: DXF import carrying native-identifier provenance.
/// - `imported`: DXF import carrying detached native-handle provenance.
///
/// # Returns
/// IDs keyed by the exact decimal x coordinate of each line's start point.
fn ids_by_line_start(entities: &[DrawingEntity]) -> BTreeMap<String, EntityId> {
    entities
        .iter()
        .filter_map(|entity| match &entity.geometry {
            CadEntity::Line { start, .. } => Some((start.x.clone(), entity.id)),
            _ => None,
        })
        .collect()
}

/// Returns native handles indexed by exact line start coordinate.
///
/// # Parameters
/// - `entities`: Snapshot-bound CAD entities to inspect.
/// - `imported`: DXF import carrying native-identifier provenance.
///
/// # Returns
/// Native handles keyed by each line's exact decimal start x coordinate.
fn handles_by_line_start(
    entities: &[DrawingEntity],
    imported: &DxfImport,
) -> BTreeMap<String, String> {
    entities
        .iter()
        .filter_map(|entity| match &entity.geometry {
            CadEntity::Line { start, .. } => imported
                .provider_import
                .entity_identifier(entity.id)
                .map(|identifier| (start.x.clone(), identifier.value.clone())),
            _ => None,
        })
        .collect()
}

/// Preserves project semantic IDs when a DXF provider changes native handles.
#[test]
fn project_import_reconciles_dxf_entities_by_structural_evidence() {
    let root = create_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan source").project.id;
    service.import(project).expect("initial import");
    let drawing_id = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("semantic drawing")
        .entity;
    let initial_snapshot = service.workspace(project).expect("workspace").snapshot;
    let initial = service
        .cad_entities(project, initial_snapshot, drawing_id, 0, 20)
        .expect("initial entities");
    let initial_ids = ids_by_line_start(&initial.entities);
    let source = root.join("drawing.dxf");
    let initial_import =
        scriberig_domain_dxf::import_dxf(&fs::read(&source).expect("initial source"), drawing_id)
            .expect("initial provider import");
    let initial_handles = handles_by_line_start(&initial.entities, &initial_import);
    assert_eq!(initial_ids.len(), 2);

    let mut drawing = dxf::Drawing::load_file(&source).expect("load source drawing");
    for (index, entity) in drawing.entities_mut().enumerate() {
        entity.common.handle = dxf::Handle(0xA000 + index as u64);
    }
    drawing.save_file(&source).expect("write changed handles");
    service.scan(None, &root).expect("rescan changed source");
    service.import(project).expect("reimport changed source");

    let next_snapshot = service
        .workspace(project)
        .expect("updated workspace")
        .snapshot;
    let updated = service
        .cad_entities(project, next_snapshot, drawing_id, 0, 20)
        .expect("updated entities");
    assert_eq!(ids_by_line_start(&updated.entities), initial_ids);
    let updated_import =
        scriberig_domain_dxf::import_dxf(&fs::read(&source).expect("updated source"), drawing_id)
            .expect("updated provider import");
    assert_ne!(
        handles_by_line_start(&updated.entities, &updated_import),
        initial_handles
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project directory");
}
