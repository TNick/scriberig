//! Verifies QGIS layer rename authority and candidate-only app exposure.

use std::collections::BTreeSet;
use std::fs;
use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationService, Capability, CapabilityAction,
    CapabilityConstraints, CapabilityContext, CapabilityScope, OperationKind,
};
use scriberig_project::{CandidateValidation, ProjectService};
use scriberig_store_sqlite::SqliteStore;

/// Creates project-scoped Propose authority at one exact snapshot.
///
/// # Parameters
/// - `project`: Project identity to authorize.
/// - `snapshot`: Exact candidate base snapshot.
///
/// # Returns
/// Scoped application context.
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

/// Requires Propose permission and leaves live QGS bytes unchanged.
#[test]
fn qgis_layer_rename_requires_propose_and_returns_candidate() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-qgis-edit-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project root");
    let source = br#"<qgis><projectlayers><maplayer><id>layer-1</id><layername>Parcels</layername><datasource>data.gpkg</datasource></maplayer></projectlayers><layer-tree-group name="root"><layer-tree-layer id="layer-1" name="Parcels"/></layer-tree-group></qgis>"#;
    fs::write(root.join("project.qgs"), source).expect("write QGS source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open store"));
    let projects = ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan QGIS project");
    let project = scan.project.id;
    let imported = projects.import(project).expect("import QGIS project");
    let qgis = projects
        .qgis_project(project, "project.qgs", imported.snapshot)
        .expect("read exact QGIS project");
    let reference = qgis.project.layers[0].id;
    let application = ApplicationService::new(()).with_project_service(projects);
    let unauthorized = ApplicationContext {
        snapshot: Some(imported.snapshot),
        ..ApplicationContext::default()
    };
    assert!(matches!(
        application.propose_qgis_layer_rename(&unauthorized, project, reference, "Lots"),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let exact_context = ApplicationContext {
        snapshot: Some(imported.snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Entity(reference),
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
        .propose_qgis_layer_rename(&exact_context, project, reference, "Exact Lots")
        .expect("exact entity grant authorizes selected reference");
    let wrong_target_context = ApplicationContext {
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Entity(scriberig_core::EntityId::new()),
            actions: BTreeSet::from([CapabilityAction::Propose]),
            constraints: CapabilityConstraints {
                snapshot: Some(imported.snapshot),
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..exact_context
    };
    assert!(matches!(
        application.propose_qgis_layer_rename(&wrong_target_context, project, reference, "Nope"),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let candidate = application
        .propose_qgis_layer_rename(
            &propose_context(project, imported.snapshot),
            project,
            reference,
            "Lots",
        )
        .expect("authorized QGIS rename candidate");
    assert_eq!(candidate.reference, reference);
    assert_eq!(candidate.path, "project.qgs");
    assert_eq!(candidate.validation, CandidateValidation::Pending);
    assert_eq!(
        fs::read(root.join("project.qgs")).expect("live project bytes"),
        source
    );
    drop(application);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project root");
}
