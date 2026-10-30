//! Verifies local HTTP QGIS rename requests receive scoped Propose authority.

use std::sync::Arc;

use super::{HttpState, dispatch};
use crate::development::ensure_fixture;
use scriberig_app::{ApplicationService, StoreInspectionBackend};
use scriberig_core::EntityId;
use scriberig_protocol::{
    QgisLayerOrderRequest, QgisLayerRenameRequest, Response, ResponseEnvelope,
};

/// Creates a QGIS rename candidate through the local HTTP protocol route.
#[test]
fn http_qgis_layer_rename_uses_local_project_propose_grant() {
    let root = std::env::temp_dir().join(format!("scriberig-http-qgis-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("create project");
    let source = br#"<qgis><projectlayers><maplayer><id>layer-1</id><layername>Parcels</layername><datasource>parcels.gpkg</datasource></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-layer id="layer-1" name="Parcels"/></layer-tree-group></qgis>"#;
    let path = root.join("project.qgs");
    std::fs::write(&path, source).expect("write QGS");
    let database = root.with_extension("sqlite");
    let store = Arc::new(scriberig_store_sqlite::SqliteStore::open(&database).expect("database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let imported = projects.import(scan.project.id).expect("import project");
    let reference = store
        .project_entities(scan.project.id)
        .expect("semantic ownership")
        .into_iter()
        .find(|entity| entity.kind == "qgis.layer_reference")
        .expect("QGIS layer reference")
        .entity;
    let service = ApplicationService::new(StoreInspectionBackend::new(Arc::clone(&store)))
        .with_project_service(projects);
    let state = HttpState {
        service,
        principal: scriberig_app::Principal::new("local-user"),
        fixture: ensure_fixture(&store).expect("development fixture"),
        allowed_origin: None,
        project_store: Some(Arc::clone(&store)),
        allowed_project_roots: vec![root.clone()],
        plan_lifecycle_handler: None,
        agent_execution_handler: None,
        provider_readiness_handler: None,
    };
    let request = QgisLayerRenameRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        reference: reference.to_string(),
        new_name: "Lots".into(),
    };
    let body = serde_json::to_string(&request).expect("encode request");
    let response = dispatch(
        &state,
        "qgis_layer_rename",
        &body,
        "http-qgis-rename".into(),
    )
    .expect("dispatch QGIS rename");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let candidate = match envelope.response {
        Response::QgisLayerRename(candidate) => candidate,
        other => panic!("expected QGIS rename candidate, got {other:?}"),
    };
    assert_eq!(candidate.project, scan.project.id.to_string());
    assert_eq!(candidate.base_snapshot, imported.snapshot.to_string());
    assert_eq!(candidate.reference, reference.to_string());
    assert_eq!(candidate.path, "project.qgs");
    assert_eq!(std::fs::read(&path).expect("live source"), source);

    drop(state);
    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove project");
}

/// Creates a full layer-order candidate through the local HTTP protocol route.
#[test]
fn http_qgis_layer_order_uses_local_project_propose_grant() {
    let root = std::env::temp_dir().join(format!("scriberig-http-qgis-order-{}", EntityId::new()));
    std::fs::create_dir_all(&root).expect("create project");
    let source = br#"<qgis><projectlayers><maplayer><id>layer-1</id><layername>Parcels</layername><datasource>parcels.gpkg</datasource></maplayer><maplayer><id>layer-2</id><layername>Roads</layername><datasource>roads.gpkg</datasource></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-layer id="layer-1" name="Parcels"/><layer-tree-layer id="layer-2" name="Roads"/></layer-tree-group></qgis>"#;
    let path = root.join("project.qgs");
    std::fs::write(&path, source).expect("write QGS");
    let database = root.with_extension("sqlite");
    let store = Arc::new(scriberig_store_sqlite::SqliteStore::open(&database).expect("database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let imported = projects.import(scan.project.id).expect("import project");
    let qgis = projects
        .qgis_project(scan.project.id, "project.qgs", imported.snapshot)
        .expect("read project");
    let ordered_references = qgis
        .project
        .layers
        .iter()
        .rev()
        .map(|layer| layer.id.to_string())
        .collect::<Vec<_>>();
    let service = ApplicationService::new(StoreInspectionBackend::new(Arc::clone(&store)))
        .with_project_service(projects);
    let state = HttpState {
        service,
        principal: scriberig_app::Principal::new("local-user"),
        fixture: ensure_fixture(&store).expect("development fixture"),
        allowed_origin: None,
        project_store: Some(Arc::clone(&store)),
        allowed_project_roots: vec![root.clone()],
        plan_lifecycle_handler: None,
        agent_execution_handler: None,
        provider_readiness_handler: None,
    };
    let request = QgisLayerOrderRequest {
        project: scan.project.id.to_string(),
        snapshot: imported.snapshot.to_string(),
        path: "project.qgs".into(),
        ordered_references,
    };
    let body = serde_json::to_string(&request).expect("encode request");
    let response = dispatch(&state, "qgis_layer_order", &body, "http-qgis-order".into())
        .expect("dispatch QGIS layer order");
    let envelope: ResponseEnvelope = serde_json::from_value(response).expect("typed response");
    let candidate = match envelope.response {
        Response::QgisLayerOrder(candidate) => candidate,
        other => panic!("expected QGIS layer-order candidate, got {other:?}"),
    };
    assert_eq!(candidate.project, scan.project.id.to_string());
    assert_eq!(candidate.base_snapshot, imported.snapshot.to_string());
    assert_eq!(candidate.path, "project.qgs");
    assert_eq!(std::fs::read(&path).expect("live source"), source);

    drop(state);
    drop(store);
    std::fs::remove_file(database).expect("remove database");
    std::fs::remove_dir_all(root).expect("remove project");
}
