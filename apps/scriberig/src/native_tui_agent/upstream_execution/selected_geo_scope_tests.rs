//! Verifies geospatial agent capability follows trusted exact semantic selection.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use rusqlite::Connection;
use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityId, CapabilityScope, OperationKind,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use super::for_selected_geo_entity;

/// Exposes Geo tools only for an exact selected `GeoPackage` entity and snapshot.
#[test]
fn geo_tools_require_selected_geopackage_entity_and_scope_proposal() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let source = directory.path().join("parcels.gpkg");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/parcels.gpkg");
    fs::copy(fixture, &source).expect("copy GeoPackage fixture");
    let database = directory.path().join("store.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, directory.path())
        .expect("scan project")
        .project
        .id;
    let snapshot = projects.import(project).expect("import project").snapshot;
    let selected = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "geo.layer")
        .expect("geospatial layer");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::from([
                CapabilityAction::Custom("geo.set_attribute".to_owned()),
                CapabilityAction::Custom("geo.replace_geometry".to_owned()),
                CapabilityAction::Custom("artifact.inspect".to_owned()),
            ]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    };
    let scope = ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: selected.entity.to_string(),
        selection: Some(selected.entity.to_string()),
    };

    let granted = for_selected_geo_entity(&context, Some(&scope), &store, &projects);
    assert_selected_geo_scope(
        &granted,
        selected.entity,
        project.to_string().as_str(),
        snapshot,
    );
    let unselected = for_selected_geo_entity(&context, None, &store, &projects);
    assert_unselected_geo_scope(&unselected);
}

/// Verifies selected Geo authority is entity- and snapshot-scoped.
///
/// # Parameters
/// - `context`: Scoped branch context returned by the capability filter.
/// - `entity`: Exact selected Geo layer identity.
/// - `project`: Exact active project identity.
/// - `snapshot`: Exact active snapshot.
fn assert_selected_geo_scope(
    context: &ApplicationContext,
    entity: EntityId,
    project: &str,
    snapshot: scriberig_core::SnapshotId,
) {
    let visible = visible_tool_names(context);
    assert!(visible.iter().any(|name| name == "geo.set_attribute"));
    assert!(visible.iter().any(|name| name == "artifact.inspect"));
    assert!(visible.iter().any(|name| name == "geo.replace_geometry"));
    assert!(context.capabilities.permits(
        &CapabilityAction::Custom("geo.set_attribute".to_owned()),
        &CapabilityScope::Entity(entity),
        None,
        None,
    ));
    assert!(context.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Entity(entity),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
    assert!(!context.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Custom(format!("project:{project}")),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
    assert!(!context.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Custom("project:another-project".to_owned()),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
    assert!(context.capabilities.permits(
        &CapabilityAction::Inspect,
        &CapabilityScope::Custom(format!("project:{project}")),
        Some(snapshot),
        Some(OperationKind::Inspect),
    ));
    assert!(!context.capabilities.permits(
        &CapabilityAction::Inspect,
        &CapabilityScope::Custom("project:another-project".to_owned()),
        Some(snapshot),
        Some(OperationKind::Inspect),
    ));
}

/// Verifies no Geo or artifact tools remain without an exact selected scope.
///
/// # Parameters
/// - `context`: Scoped branch context returned when selection is absent.
fn assert_unselected_geo_scope(context: &ApplicationContext) {
    let visible = visible_tool_names(context);
    assert!(!visible.iter().any(|name| name == "geo.set_attribute"));
    assert!(!visible.iter().any(|name| name == "artifact.inspect"));
    assert!(!visible.iter().any(|name| name == "geo.replace_geometry"));
    assert!(!context.capabilities.permits(
        &CapabilityAction::Custom("geo.set_attribute".to_owned()),
        &CapabilityScope::Project,
        None,
        None,
    ));
}

/// Lists tools visible to one scoped capability context.
///
/// # Parameters
/// - `context`: Capability context to inspect.
///
/// # Returns
/// Tool names visible through the child runtime catalog.
fn visible_tool_names(context: &ApplicationContext) -> Vec<String> {
    crate::child_runtime::inspect_catalog()
        .descriptors_for(&context.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect()
}

/// Hides Geo edit tools when the selected layer has preservation-sensitive triggers.
#[test]
fn geo_edit_tools_require_selected_layer_writer_capability() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let source = directory.path().join("parcels.gpkg");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/parcels.gpkg");
    fs::copy(fixture, &source).expect("copy GeoPackage fixture");
    Connection::open(&source)
        .expect("open GeoPackage fixture")
        .execute_batch("CREATE TRIGGER parcel_audit AFTER UPDATE ON parcels BEGIN SELECT 1; END;")
        .expect("add preservation-sensitive trigger");
    let database = directory.path().join("store.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, directory.path())
        .expect("scan project")
        .project
        .id;
    let snapshot = projects.import(project).expect("import project").snapshot;
    let layer = store
        .project_entities(project)
        .expect("project entities")
        .into_iter()
        .find(|record| record.kind == "geo.layer")
        .expect("GeoPackage layer");
    let catalog = projects
        .geopackage_catalog(project, "parcels.gpkg", snapshot, 0, 10)
        .expect("inspect read-only layer");
    assert!(!catalog.layers[0].editable);
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::from([
                CapabilityAction::Propose,
                CapabilityAction::Custom("geo.set_attribute".to_owned()),
                CapabilityAction::Custom("geo.replace_geometry".to_owned()),
            ]),
            constraints: CapabilityConstraints {
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    };
    let scope = ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: layer.entity.to_string(),
        selection: Some(layer.entity.to_string()),
    };

    let scoped = for_selected_geo_entity(&context, Some(&scope), &store, &projects);
    let visible_tools = crate::child_runtime::inspect_catalog()
        .descriptors_for(&scoped.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert!(!visible_tools.iter().any(|name| name == "geo.set_attribute"));
    assert!(
        !visible_tools
            .iter()
            .any(|name| name == "geo.replace_geometry")
    );
    assert!(!scoped.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Custom(format!("project:{project}")),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
}

/// Exposes bounded spreadsheet range reads only for a selected XLSX workbook.
#[test]
fn spreadsheet_range_read_requires_selected_workbook_and_exact_snapshot() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let mut workbook = rxls::Workbook::new();
    let sheet = workbook.add_sheet("Budget");
    sheet.write_number(0, 0, 42.0);
    sheet.write_number(0, 1, 24.0);
    fs::write(
        directory.path().join("budget.xlsx"),
        workbook.to_xlsx_checked().expect("valid XLSX fixture"),
    )
    .expect("write workbook fixture");
    let database = directory.path().join("store.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, directory.path())
        .expect("scan workbook")
        .project
        .id;
    let snapshot = projects.import(project).expect("import workbook").snapshot;
    let workbook = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "spreadsheet.workbook")
        .expect("spreadsheet workbook");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::from([CapabilityAction::Custom(
                "spreadsheet.read_range".to_owned(),
            )]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    };
    let selected = ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: workbook.entity.to_string(),
        selection: Some(workbook.entity.to_string()),
    };
    let granted = for_selected_geo_entity(&context, Some(&selected), &store, &projects);
    let descriptors = crate::child_runtime::inspect_catalog()
        .descriptors_for(&granted.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert!(
        descriptors
            .iter()
            .any(|name| name == "spreadsheet.read_range")
    );
    assert!(granted.capabilities.permits(
        &CapabilityAction::Custom("spreadsheet.read_range".to_owned()),
        &CapabilityScope::Custom(format!("project:{project}")),
        Some(snapshot),
        Some(OperationKind::Inspect),
    ));

    let unselected = for_selected_geo_entity(&context, None, &store, &projects);
    let descriptors = crate::child_runtime::inspect_catalog()
        .descriptors_for(&unselected.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert!(
        !descriptors
            .iter()
            .any(|name| name == "spreadsheet.read_range")
    );
    assert!(!unselected.capabilities.permits(
        &CapabilityAction::Custom("spreadsheet.read_range".to_owned()),
        &CapabilityScope::Project,
        None,
        None,
    ));

    let stale = ProjectSemanticScope {
        snapshot: EntityId::new().to_string(),
        ..selected
    };
    let stale_context = for_selected_geo_entity(&context, Some(&stale), &store, &projects);
    assert!(
        !stale_context
            .capabilities
            .allows(&CapabilityId::new("spreadsheet.read_range").expect("valid tool id"))
    );
}

#[path = "selected_document_scope_tests.rs"]
mod document_tests;
#[path = "selected_rich_scope_tests.rs"]
mod rich_tests;
