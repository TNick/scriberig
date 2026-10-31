//! Verifies snapshot-bound CAD selection bounds and drawing-space isolation.

use std::fs;
use std::sync::Arc;

use scriberig_domain_cad::DrawingSpace;
use scriberig_project::{CadBounds2D, CadBoundsQueryRequest, ProjectService};
use scriberig_store_sqlite::SqliteStore;

/// Queries semantic line extents in the exact imported DXF snapshot.
#[test]
fn cad_bounds_query_is_snapshot_and_space_scoped() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-cad-bounds-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut drawing = dxf::Drawing::new();
    drawing.add_entity(dxf::entities::Entity::new(dxf::entities::EntityType::Line(
        dxf::entities::Line {
            p1: dxf::Point::new(0.0, 0.0, 0.0),
            p2: dxf::Point::new(10.0, 0.0, 0.0),
            ..Default::default()
        },
    )));
    drawing.add_entity(dxf::entities::Entity::new(dxf::entities::EntityType::Line(
        dxf::entities::Line {
            p1: dxf::Point::new(20.0, 0.0, 0.0),
            p2: dxf::Point::new(30.0, 0.0, 0.0),
            ..Default::default()
        },
    )));
    drawing
        .save_file(root.join("drawing.dxf"))
        .expect("write DXF");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let project = service.scan(None, &root).expect("scan").project.id;
    service.import(project).expect("import DXF");
    let snapshot = service.workspace(project).expect("workspace").snapshot;
    let drawing_id = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("drawing")
        .entity;
    let bounds = CadBounds2D {
        min_x: "9".to_owned(),
        min_y: "-1".to_owned(),
        max_x: "11".to_owned(),
        max_y: "1".to_owned(),
    };
    let page = service
        .cad_entities_intersecting_bounds(CadBoundsQueryRequest {
            project,
            snapshot,
            drawing: drawing_id,
            space: DrawingSpace::Model,
            bounds: bounds.clone(),
            offset: 0,
            limit: 10,
        })
        .expect("query model-space bounds");
    assert_eq!(page.total, 1);
    assert!(matches!(
        &page.entities[0].geometry,
        scriberig_domain_cad::CadEntity::Line { start, end }
            if start.x == "0" && end.x == "10"
    ));
    assert!(page.complete);
    assert_eq!(page.unbounded_entities, 0);

    let other_space = service
        .cad_entities_intersecting_bounds(CadBoundsQueryRequest {
            project,
            snapshot,
            drawing: drawing_id,
            space: DrawingSpace::Named("paper-preview".to_owned()),
            bounds,
            offset: 0,
            limit: 10,
        })
        .expect("query a separate drawing space");
    assert_eq!(other_space.total, 0);
    assert!(
        service
            .cad_entities_intersecting_bounds(CadBoundsQueryRequest {
                project,
                snapshot,
                drawing: drawing_id,
                space: DrawingSpace::Model,
                bounds: CadBounds2D {
                    min_x: "2".to_owned(),
                    min_y: "2".to_owned(),
                    max_x: "1".to_owned(),
                    max_y: "3".to_owned(),
                },
                offset: 0,
                limit: 10,
            })
            .is_err()
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}
