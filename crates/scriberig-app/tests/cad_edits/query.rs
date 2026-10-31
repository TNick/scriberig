//! Verifies exact-snapshot CAD query authorization and result scoping.

use super::*;

#[allow(
    clippy::too_many_lines,
    reason = "This application integration test verifies layer-scoped inspection and search results against an imported drawing."
)]
/// Routes layer-filtered reads and text search through exact-snapshot Inspect grants.
#[test]
fn cad_text_and_layer_queries_require_inspect_authority() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-cad-query-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut drawing = dxf::Drawing::new();
    drawing.add_layer(dxf::tables::Layer {
        name: "LABELS".to_owned(),
        ..Default::default()
    });
    let mut label =
        dxf::entities::Entity::new(dxf::entities::EntityType::Text(dxf::entities::Text {
            value: "Parcel 124".to_owned(),
            location: dxf::Point::new(2.0, 3.0, 0.0),
            ..Default::default()
        }));
    label.common.layer = "LABELS".to_owned();
    drawing.add_entity(label);
    drawing
        .save_file(root.join("drawing.dxf"))
        .expect("write DXF");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let project = projects.scan(None, &root).expect("scan").project.id;
    let imported = projects.import(project).expect("import");
    let drawing_id = store
        .project_entities(project)
        .expect("owned entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.drawing")
        .expect("drawing")
        .entity;
    let application = ApplicationService::new(()).with_project_service(projects);
    let untrusted = ApplicationContext::default().with_snapshot(imported.snapshot);
    assert!(matches!(
        application.find_cad_text(
            &untrusted,
            &CadTextSearchRequest {
                project,
                snapshot: imported.snapshot,
                drawing: drawing_id,
                query: "parcel".to_owned(),
                offset: 0,
                limit: 10,
            },
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let context = inspect_context(project, imported.snapshot);
    let matches = application
        .find_cad_text(
            &context,
            &CadTextSearchRequest {
                project,
                snapshot: imported.snapshot,
                drawing: drawing_id,
                query: "parcel 124".to_owned(),
                offset: 0,
                limit: 10,
            },
        )
        .expect("authorized text search");
    assert_eq!(matches.total, 1);
    assert_eq!(matches.matches[0].text, "Parcel 124");
    let bounds_request = CadBoundsQueryRequest {
        project,
        snapshot: imported.snapshot,
        drawing: drawing_id,
        space: DrawingSpace::Model,
        bounds: CadBounds2D {
            min_x: "0".to_owned(),
            min_y: "0".to_owned(),
            max_x: "5".to_owned(),
            max_y: "5".to_owned(),
        },
        offset: 0,
        limit: 10,
    };
    assert!(matches!(
        application.read_cad_entities_intersecting_bounds(&untrusted, bounds_request.clone()),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let bounded = application
        .read_cad_entities_intersecting_bounds(&context, bounds_request)
        .expect("authorized bounds query");
    assert_eq!(bounded.total, 1);
    assert!(matches!(
        &bounded.entities[0].geometry,
        scriberig_domain_cad::CadEntity::Text { value, .. } if value == "Parcel 124"
    ));
    let layer = application
        .inspect_cad_drawing(&context, project, imported.snapshot, drawing_id)
        .expect("authorized drawing summary")
        .layers
        .into_iter()
        .find(|layer| layer.name == "LABELS")
        .expect("labels layer");
    let entities = application
        .read_cad_entities_by_layer(
            &context,
            &CadLayerEntitiesRequest {
                project,
                snapshot: imported.snapshot,
                drawing: drawing_id,
                layer: layer.id,
                offset: 0,
                limit: 10,
            },
        )
        .expect("authorized layer query");
    assert_eq!(entities.total, 1);
    assert_eq!(entities.entities[0].layer, layer.id);
    drop(application);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
