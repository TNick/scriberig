//! Verifies integrated agents create candidate-only QGIS rename operations.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_agent_core::Tool as _;
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationService, Capability,
    CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope, OperationKind,
    StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;

/// Creates Propose authority for one selected project and exact snapshot.
///
/// # Parameters
/// - `project`: Project identity to authorize.
/// - `snapshot`: Exact candidate base snapshot.
///
/// # Returns
/// Project and tool-scoped application context.
fn agent_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([
            Capability {
                scope: CapabilityScope::Project,
                actions: BTreeSet::from([CapabilityAction::Custom(
                    "qgis.rename_layer_reference".to_owned(),
                )]),
                constraints: CapabilityConstraints::unrestricted(),
            },
            Capability {
                scope: CapabilityScope::Custom(format!("project:{project}")),
                actions: BTreeSet::from([CapabilityAction::Propose]),
                constraints: CapabilityConstraints {
                    operation: Some(OperationKind::DomainOperation),
                    ..CapabilityConstraints::unrestricted()
                },
            },
        ]),
        ..ApplicationContext::default()
    }
}

/// Returns a QGIS candidate handle without changing the live project file.
#[test]
fn qgis_agent_tool_uses_propose_grant_and_preserves_live_bytes() {
    let root = std::env::temp_dir().join(format!("scriberig-agent-qgis-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("project root");
    let path = root.join("project.qgs");
    let bytes = br#"<qgis><projectlayers><maplayer><id>layer-1</id><layername>Parcels</layername><datasource>parcels.gpkg</datasource></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-layer id="layer-1" name="Parcels"/></layer-tree-group></qgis>"#;
    fs::write(&path, bytes).expect("write QGS");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let imported = projects.import(project).expect("import project");
    let reference = store
        .project_entities(project)
        .expect("semantic entities")
        .into_iter()
        .find(|entity| entity.kind == "qgis.layer_reference")
        .expect("layer reference")
        .entity;
    let service = ApplicationService::new(ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    ))
    .with_project_service(projects.clone());
    let tool = LocalAgentTool::new(&service).with_project_store(Arc::clone(&store));
    let result = tool
        .invoke_named(
            &agent_context(project, imported.snapshot),
            "qgis.rename_layer_reference",
            &serde_json::json!({
                "project": project.to_string(),
                "reference": reference.to_string(),
                "new_name": "Lots"
            }),
        )
        .expect("QGIS rename candidate");
    assert_eq!(result.entity, reference);
    assert_eq!(result.snapshot, Some(imported.snapshot));
    assert_eq!(result.object_type.as_deref(), Some("qgis.candidate"));
    assert_eq!(result.handles.len(), 1);
    let candidate = projects
        .candidate(&result.handles[0])
        .expect("load durable candidate")
        .expect("candidate exists");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("qgis.rename_layer_reference")
    );
    assert_eq!(fs::read(&path).expect("live QGS bytes"), bytes);

    drop(tool);
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project root");
}
