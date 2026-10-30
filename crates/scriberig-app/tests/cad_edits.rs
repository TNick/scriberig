//! Verifies CAD edit authorization and candidate-only app behavior.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_agent_core::Tool;
use scriberig_app::agent_tools::{
    CAD_LAYER_CHANGE_TOOL, CAD_LINE_EDIT_TOOL, CadLayerChangeTool, CadLineEditTool,
};
use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationService, CadLayerEntitiesRequest,
    CadTextSearchRequest, Capability, CapabilityAction, CapabilityConstraints, CapabilityContext,
    CapabilityScope, OperationKind,
};
use scriberig_domain_cad::{Coordinate, DrawingEntity, DrawingSpace};
use scriberig_project::{CadBounds2D, CadBoundsQueryRequest, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

/// Creates a project-scoped Propose context for one snapshot.
///
/// # Parameters
/// - `project`: Project identity authorized by the grant.
/// - `snapshot`: Exact snapshot identity bound by the grant.
///
/// # Returns
/// Context constrained to domain proposals for the project snapshot.
fn propose_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Propose]),
            constraints: CapabilityConstraints {
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    }
}

/// Creates an exact-snapshot Inspect context scoped to one project.
///
/// # Parameters
/// - `project`: Authorized project identity.
/// - `snapshot`: Exact immutable snapshot constraint.
///
/// # Returns
/// Scoped CAD inspection context.
fn inspect_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Inspect]),
            constraints: CapabilityConstraints {
                operation: Some(OperationKind::Inspect),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "This application integration test verifies exact-snapshot CAD authorization, candidate persistence, and no live write across the complete flow."
)]
/// Routes a line edit through application authorization and creates no live write.
#[test]
fn cad_line_edit_requires_propose_grant_and_returns_candidate() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-cad-edit-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut drawing = dxf::Drawing::new();
    drawing.add_layer(dxf::tables::Layer {
        name: "LABELS".to_owned(),
        ..Default::default()
    });
    drawing.add_entity(dxf::entities::Entity::new(dxf::entities::EntityType::Line(
        dxf::entities::Line {
            p1: dxf::Point::new(0.0, 0.0, 0.0),
            p2: dxf::Point::new(10.0, 0.0, 0.0),
            ..Default::default()
        },
    )));
    let mut original = Vec::new();
    drawing.save(&mut original).expect("serialize DXF");
    let source = root.join("drawing.dxf");
    fs::write(&source, &original).expect("write DXF");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let project = projects.scan(None, &root).expect("scan").project.id;
    let imported = projects.import(project).expect("import");
    let entity = store
        .project_entities(project)
        .expect("owned entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.entity")
        .expect("line ownership");
    let revision = store
        .resolve(imported.snapshot, entity.entity)
        .expect("resolve line")
        .expect("line revision");
    let payload = store.get(revision).expect("load line").expect("payload");
    let _: DrawingEntity = serde_json::from_slice(payload.payload()).expect("decode line");
    let application = ApplicationService::new(()).with_project_service(projects);
    let untrusted = ApplicationContext::default().with_snapshot(imported.snapshot);
    assert!(matches!(
        application.inspect_cad_drawing(&untrusted, project, imported.snapshot, {
            let record = store
                .project_entities(project)
                .expect("entities")
                .into_iter()
                .find(|owned| owned.kind == "cad.drawing")
                .expect("drawing");
            record.entity
        }),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let drawing = store
        .project_entities(project)
        .expect("entities")
        .into_iter()
        .find(|owned| owned.kind == "cad.drawing")
        .expect("drawing");
    let view = application
        .inspect_cad_drawing(
            &inspect_context(project, imported.snapshot),
            project,
            imported.snapshot,
            drawing.entity,
        )
        .expect("authorized CAD drawing read");
    assert_eq!(view.entity_count, 1);
    let destination_layer = view
        .layers
        .iter()
        .find(|layer| layer.name == "LABELS")
        .expect("destination layer")
        .id;
    assert_eq!(
        application
            .read_cad_entities(
                &inspect_context(project, imported.snapshot),
                project,
                imported.snapshot,
                drawing.entity,
                0,
                1,
            )
            .expect("authorized CAD page")
            .entities
            .len(),
        1
    );
    assert!(matches!(
        application.propose_cad_line_geometry(
            &untrusted,
            project,
            entity.entity,
            &Coordinate::new("1", "2", "0"),
            &Coordinate::new("3", "4", "0"),
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    assert!(matches!(
        application
            .propose_cad_layer_change(&untrusted, project, entity.entity, destination_layer,),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let exact_entity_context = ApplicationContext {
        snapshot: Some(imported.snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Entity(entity.entity),
            actions: BTreeSet::from([CapabilityAction::Propose]),
            constraints: CapabilityConstraints {
                snapshot: Some(imported.snapshot),
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::default()
    };
    application
        .propose_cad_line_geometry(
            &exact_entity_context,
            project,
            entity.entity,
            &Coordinate::new("2", "3", "0"),
            &Coordinate::new("12", "13", "0"),
        )
        .expect("exact entity grant authorizes selected CAD target");
    assert!(matches!(
        application.propose_cad_line_geometry(
            &exact_entity_context,
            project,
            scriberig_core::EntityId::new(),
            &Coordinate::new("2", "3", "0"),
            &Coordinate::new("12", "13", "0"),
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let candidate = application
        .propose_cad_line_geometry(
            &propose_context(project, imported.snapshot),
            project,
            entity.entity,
            &Coordinate::new("1", "2", "0"),
            &Coordinate::new("3", "4", "0"),
        )
        .expect("authorized CAD candidate");
    assert_eq!(candidate.base_snapshot, imported.snapshot);
    assert_eq!(candidate.path, "drawing.dxf");
    let layer_candidate = application
        .propose_cad_layer_change(
            &propose_context(project, imported.snapshot),
            project,
            entity.entity,
            destination_layer,
        )
        .expect("authorized CAD layer candidate");
    assert_eq!(layer_candidate.base_snapshot, imported.snapshot);
    assert_eq!(layer_candidate.path, "drawing.dxf");
    let tool = CadLineEditTool::new(
        ApplicationService::new(()).with_project_service(ProjectService::new(Arc::clone(&store))),
        project,
    );
    let arguments = serde_json::json!({
        "entity": entity.entity.to_string(),
        "start": {"x": "2", "y": "3", "z": "0"},
        "end": {"x": "12", "y": "13", "z": "0"},
    });
    assert!(matches!(
        tool.invoke_named(&untrusted, CAD_LINE_EDIT_TOOL, &arguments),
        Err(scriberig_agent_core::ToolError::CapabilityDenied { .. })
    ));
    let tool_candidate = tool
        .invoke_named(
            &propose_context(project, imported.snapshot),
            CAD_LINE_EDIT_TOOL,
            &arguments,
        )
        .expect("authorized agent CAD candidate");
    assert_eq!(tool_candidate.entity, entity.entity);
    assert_eq!(tool_candidate.handles.len(), 1);
    let layer_tool = CadLayerChangeTool::new(
        ApplicationService::new(()).with_project_service(ProjectService::new(Arc::clone(&store))),
        project,
    );
    let layer_arguments = serde_json::json!({
        "entity": entity.entity.to_string(),
        "destination_layer": destination_layer.to_string(),
    });
    assert!(matches!(
        layer_tool.invoke_named(&untrusted, CAD_LAYER_CHANGE_TOOL, &layer_arguments),
        Err(scriberig_agent_core::ToolError::CapabilityDenied { .. })
    ));
    let layer_tool_candidate = layer_tool
        .invoke_named(
            &propose_context(project, imported.snapshot),
            CAD_LAYER_CHANGE_TOOL,
            &layer_arguments,
        )
        .expect("authorized agent CAD layer candidate");
    assert_eq!(layer_tool_candidate.entity, entity.entity);
    assert_eq!(layer_tool_candidate.handles.len(), 1);
    assert_eq!(fs::read(&source).expect("live source"), original);
    drop(application);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

#[path = "cad_edits/query.rs"]
mod query;
