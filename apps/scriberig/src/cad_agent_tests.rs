//! Verifies integrated provider CAD tools use exact selected entity grants.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_agent_core::Tool as _;
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, Capability,
    CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityId, CapabilityScope,
    OperationKind, StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::EntityId;
use scriberig_domain_cad::{CadEntity, DrawingEntity};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;

/// Creates provider CAD candidates under one exact entity Propose grant.
#[test]
fn product_cad_tools_return_candidates_for_the_selected_entity() {
    let root = std::env::temp_dir().join(format!("scriberig-agent-cad-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("create project directory");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/drawing.dxf");
    fs::copy(&fixture, root.join("drawing.dxf")).expect("copy DXF fixture");
    let source = fs::read(root.join("drawing.dxf")).expect("read copied DXF");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, &root)
        .expect("scan DXF project")
        .project
        .id;
    let snapshot = projects
        .import(project)
        .expect("import DXF project")
        .snapshot;
    let entity = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .filter(|record| record.kind == "cad.entity")
        .find_map(|record| {
            let revision = store.resolve(snapshot, record.entity).ok().flatten()?;
            let revision = store.get(revision).ok().flatten()?;
            let entity: DrawingEntity = serde_json::from_slice(revision.payload()).ok()?;
            matches!(entity.geometry, CadEntity::Line { .. }).then_some(record)
        })
        .expect("CAD line entity");
    let destination_layer = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "cad.layer")
        .expect("CAD layer");
    let service = ApplicationService::new(ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    ))
    .with_project_service(projects);
    let tool = LocalAgentTool::new(&service);
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext {
            granted: ["cad.update_line_geometry", "cad.change_entity_layer"]
                .into_iter()
                .map(|name| CapabilityId::new(name).expect("CAD tool capability"))
                .collect(),
            grants: [Capability {
                scope: CapabilityScope::Entity(entity.entity),
                actions: BTreeSet::from([CapabilityAction::Propose]),
                constraints: CapabilityConstraints {
                    snapshot: Some(snapshot),
                    operation: Some(OperationKind::DomainOperation),
                    ..CapabilityConstraints::unrestricted()
                },
            }]
            .into_iter()
            .collect(),
        },
        ..ApplicationContext::default()
    };
    let geometry = tool
        .invoke_named(
            &context,
            "cad.update_line_geometry",
            &serde_json::json!({
                "project": project.to_string(),
                "entity": entity.entity.to_string(),
                "start": {"x": "1", "y": "2", "z": "0"},
                "end": {"x": "3", "y": "4", "z": "0"}
            }),
        )
        .expect("line geometry candidate");
    assert_eq!(geometry.entity, entity.entity);
    assert_eq!(geometry.snapshot, Some(snapshot));
    assert_eq!(geometry.object_type.as_deref(), Some("cad.candidate"));
    let layer = tool
        .invoke_named(
            &context,
            "cad.change_entity_layer",
            &serde_json::json!({
                "project": project.to_string(),
                "entity": entity.entity.to_string(),
                "destination_layer": destination_layer.entity.to_string()
            }),
        )
        .expect("layer change candidate");
    assert_eq!(layer.entity, entity.entity);
    assert_eq!(layer.snapshot, Some(snapshot));
    assert_eq!(
        fs::read(root.join("drawing.dxf")).expect("live DXF"),
        source
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove project store file");
    fs::remove_dir_all(root).expect("remove temporary project");
}
