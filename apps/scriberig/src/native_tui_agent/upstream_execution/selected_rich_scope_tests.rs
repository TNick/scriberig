//! Verifies selected rich-document tools follow exact target and fidelity grants.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_agent_runtime::context::ProjectSemanticScope;
use scriberig_app::{
    ApplicationContext, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityId, CapabilityScope, OperationKind,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use super::for_selected_geo_entity;

/// Verifies QGIS rename tools require the selected reference at the exact snapshot.
#[test]
fn qgis_rename_tool_requires_selected_reference_and_exact_snapshot() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    fs::write(
        directory.path().join("project.qgs"),
        br#"<qgis><projectlayers><maplayer><id>layer-1</id><layername>Parcels</layername><datasource>parcels.gpkg</datasource></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-layer id="layer-1" name="Parcels"/></layer-tree-group></qgis>"#,
    )
    .expect("write QGIS project");
    let database = directory.path().join("store.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, directory.path())
        .expect("scan QGIS project")
        .project
        .id;
    let snapshot = projects
        .import(project)
        .expect("import QGIS project")
        .snapshot;
    let reference = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "qgis.layer_reference")
        .expect("QGIS layer reference");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::from([
                CapabilityAction::Propose,
                CapabilityAction::Custom("qgis.rename_layer_reference".to_owned()),
            ]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    };
    let selected = ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: reference.entity.to_string(),
        selection: Some(reference.entity.to_string()),
    };

    let scoped = for_selected_geo_entity(&context, Some(&selected), &store, &projects);
    let visible = crate::child_runtime::inspect_catalog()
        .descriptors_for(&scoped.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert!(
        visible
            .iter()
            .any(|name| name == "qgis.rename_layer_reference")
    );
    assert!(scoped.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Entity(reference.entity),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
    assert!(!scoped.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Project,
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));

    let unselected = for_selected_geo_entity(&context, None, &store, &projects);
    assert!(
        !crate::child_runtime::inspect_catalog()
            .descriptors_for(&unselected.capabilities)
            .iter()
            .any(|descriptor| descriptor.definition.name == "qgis.rename_layer_reference")
    );

    let stale = ProjectSemanticScope {
        snapshot: EntityId::new().to_string(),
        ..selected
    };
    let stale_context = for_selected_geo_entity(&context, Some(&stale), &store, &projects);
    assert!(
        !crate::child_runtime::inspect_catalog()
            .descriptors_for(&stale_context.capabilities)
            .iter()
            .any(|descriptor| descriptor.definition.name == "qgis.rename_layer_reference")
    );
}

/// Exposes generic CAD operations only for an editable selected DXF entity.
#[test]
fn generic_domain_operation_requires_selected_editable_artifact_and_exact_snapshot() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/drawing.dxf");
    fs::copy(fixture, directory.path().join("drawing.dxf")).expect("copy DXF fixture");
    let database = directory.path().join("store.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, directory.path())
        .expect("scan CAD project")
        .project
        .id;
    let snapshot = projects
        .import(project)
        .expect("import CAD project")
        .snapshot;
    let entity = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "cad.entity")
        .expect("CAD semantic entity");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::from([
                CapabilityAction::Custom("domain_operation".to_owned()),
                CapabilityAction::Custom("cad.update_line_geometry".to_owned()),
                CapabilityAction::Custom("cad.change_entity_layer".to_owned()),
            ]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    };
    let selected = ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: entity.entity.to_string(),
        selection: Some(entity.entity.to_string()),
    };

    let scoped = for_selected_geo_entity(&context, Some(&selected), &store, &projects);
    assert!(
        scoped
            .capabilities
            .allows(&CapabilityId::new("domain_operation").expect("generic operation capability"))
    );
    assert!(scoped.capabilities.permits(
        &CapabilityAction::Custom("domain_operation".to_owned()),
        &CapabilityScope::Entity(entity.entity),
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
    assert!(!scoped.capabilities.permits(
        &CapabilityAction::Custom("domain_operation".to_owned()),
        &CapabilityScope::Project,
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
    let visible = crate::child_runtime::inspect_catalog()
        .descriptors_for(&scoped.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert!(
        visible
            .iter()
            .any(|name| name == "cad.update_line_geometry")
    );
    assert!(visible.iter().any(|name| name == "cad.change_entity_layer"));

    let unselected = for_selected_geo_entity(&context, None, &store, &projects);
    assert!(
        !unselected
            .capabilities
            .allows(&CapabilityId::new("domain_operation").expect("generic operation capability"))
    );
    let visible = crate::child_runtime::inspect_catalog()
        .descriptors_for(&unselected.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert!(
        !visible
            .iter()
            .any(|name| name == "cad.update_line_geometry")
    );
    assert!(!visible.iter().any(|name| name == "cad.change_entity_layer"));
}

/// Hides CAD mutation tools and broad proposal authority for an unsupported-only DXF.
#[test]
fn read_only_cad_artifact_hides_mutation_tools_and_generic_operation() {
    let directory = tempfile::tempdir().expect("temporary project directory");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project/drawing.dxf");
    let mut drawing = fs::read_to_string(fixture).expect("read DXF fixture");
    let entities_start = drawing
        .find("  2\r\nENTITIES\r\n")
        .expect("entities section")
        + "  2\r\nENTITIES\r\n".len();
    let entities_end = drawing[entities_start..]
        .find("  0\r\nENDSEC")
        .map(|offset| entities_start + offset)
        .expect("entities section end");
    let mut entities = drawing[entities_start..entities_end].to_owned();
    for kind in ["LINE", "TEXT", "ARC", "POLYLINE", "LWPOLYLINE", "CIRCLE"] {
        let marker = format!("  0\r\n{kind}\r\n");
        while let Some(start) = entities.find(&marker) {
            entities.replace_range(start..start + marker.len(), "  0\r\nPOINT\r\n");
        }
    }
    drawing.replace_range(entities_start..entities_end, &entities);
    fs::write(directory.path().join("drawing.dxf"), drawing).expect("write mixed DXF");
    let database = directory.path().join("store.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let projects = scriberig_project::ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, directory.path())
        .expect("scan mixed DXF")
        .project
        .id;
    let snapshot = projects.import(project).expect("import mixed DXF").snapshot;
    let artifact = projects
        .artifacts(project)
        .expect("artifact fidelity")
        .into_iter()
        .find(|artifact| artifact.source.adapter == "dxf")
        .expect("DXF artifact");
    assert_eq!(
        artifact.fidelity.capability,
        scriberig_project::ArtifactCapability::ReadOnlyDerived
    );
    let entity = store
        .project_entities(project)
        .expect("project semantic entities")
        .into_iter()
        .find(|record| record.kind == "cad.drawing")
        .expect("CAD drawing");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions: BTreeSet::from([
                CapabilityAction::Propose,
                CapabilityAction::Custom("domain_operation".to_owned()),
                CapabilityAction::Custom("cad.update_line_geometry".to_owned()),
                CapabilityAction::Custom("cad.change_entity_layer".to_owned()),
            ]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    };
    let selected = ProjectSemanticScope {
        project: project.to_string(),
        snapshot: snapshot.to_string(),
        root: entity.entity.to_string(),
        selection: Some(entity.entity.to_string()),
    };
    let scoped = for_selected_geo_entity(&context, Some(&selected), &store, &projects);
    assert!(
        !scoped
            .capabilities
            .allows(&CapabilityId::new("domain_operation").expect("generic operation capability"))
    );
    let visible = crate::child_runtime::inspect_catalog()
        .descriptors_for(&scoped.capabilities)
        .into_iter()
        .map(|descriptor| descriptor.definition.name)
        .collect::<Vec<_>>();
    assert!(!visible.iter().any(|name| name.starts_with("cad.")));
    assert!(!scoped.capabilities.permits(
        &CapabilityAction::Propose,
        &CapabilityScope::Project,
        Some(snapshot),
        Some(OperationKind::DomainOperation),
    ));
}

#[path = "selected_rich_scope_tests/spreadsheet.rs"]
mod spreadsheet_tests;
