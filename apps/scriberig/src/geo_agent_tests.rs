//! Verifies integrated provider geospatial edits create candidates without writing sources.

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

/// Verifies provider `GeoPackage` edits are scoped and return a reviewable candidate.
#[test]
fn geo_agent_tool_requires_grants_and_preserves_live_package() {
    let root = std::env::temp_dir().join(format!("scriberig-agent-geo-{}", EntityId::new()));
    fs::create_dir_all(&root).expect("project root");
    let path = root.join("parcels.gpkg");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/parcels.gpkg");
    fs::copy(fixture, &path).expect("copy GeoPackage fixture");
    let original = fs::read(&path).expect("read original package");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let imported = projects.import(scan.project.id).expect("import package");
    let layer = store
        .project_entities(scan.project.id)
        .expect("project entities")
        .into_iter()
        .find(|entity| entity.kind == "geo.layer")
        .expect("geospatial layer")
        .entity;
    let feature = projects
        .geopackage_feature_page(
            scan.project.id,
            "parcels.gpkg",
            imported.snapshot,
            layer,
            0,
            1,
        )
        .expect("feature page")
        .features
        .into_iter()
        .next()
        .expect("first feature");
    let service = ApplicationService::new(ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    ))
    .with_project_service(projects);
    let tool = LocalAgentTool::new(&service);
    let arguments = serde_json::json!({
        "project": scan.project.id.to_string(),
        "path": "parcels.gpkg",
        "layer": layer.to_string(),
        "feature": feature.id.to_string(),
        "field": "area",
        "value": {"Number": "11.5"}
    });
    let unauthorized = agent_context(scan.project.id, imported.snapshot, false);
    assert!(
        tool.invoke_named(&unauthorized, "geo.set_attribute", &arguments)
            .is_err()
    );

    let authorized = agent_context(scan.project.id, imported.snapshot, true);
    let result = tool
        .invoke_named(&authorized, "geo.set_attribute", &arguments)
        .expect("semantic candidate from provider tool");

    assert_eq!(result.entity, feature.id);
    assert_eq!(result.snapshot, Some(imported.snapshot));
    assert_eq!(result.object_type.as_deref(), Some("geo.candidate"));
    assert_eq!(result.handles.len(), 1);
    let geometry_result = tool
        .invoke_named(
            &authorized,
            "geo.replace_geometry",
            &serde_json::json!({
                "project": scan.project.id.to_string(),
                "path": "parcels.gpkg",
                "layer": layer.to_string(),
                "feature": feature.id.to_string(),
                "geometry": {"Point": {"x": "2.5", "y": "3.5", "z": null, "measure": null}}
            }),
        )
        .expect("semantic geometry candidate from provider tool");
    assert_eq!(geometry_result.entity, feature.id);
    assert_eq!(geometry_result.snapshot, Some(imported.snapshot));
    assert_eq!(
        geometry_result.object_type.as_deref(),
        Some("geo.candidate")
    );
    assert_eq!(geometry_result.handles.len(), 1);
    assert_eq!(
        fs::read(&path).expect("live package remains readable"),
        original
    );

    drop(tool);
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project root");
}

/// Builds a context with an exact snapshot and optional Geo operation grants.
///
/// # Parameters
/// - `project`: Project receiving the candidate.
/// - `snapshot`: Exact imported snapshot.
/// - `authorized`: Whether the provider operation capability is granted.
///
/// # Returns
/// Trusted application context for the provider operation.
fn agent_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
    authorized: bool,
) -> ApplicationContext {
    let mut grants = Vec::new();
    if authorized {
        grants.push(Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::from([
                CapabilityAction::Custom("geo.set_attribute".to_owned()),
                CapabilityAction::Custom("geo.replace_geometry".to_owned()),
            ]),
            constraints: CapabilityConstraints::unrestricted(),
        });
    }
    grants.push(Capability {
        scope: CapabilityScope::Custom(format!("project:{project}")),
        actions: BTreeSet::from([CapabilityAction::Propose]),
        constraints: CapabilityConstraints {
            operation: Some(OperationKind::DomainOperation),
            ..CapabilityConstraints::unrestricted()
        },
    });
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants(grants),
        ..ApplicationContext::default()
    }
}
