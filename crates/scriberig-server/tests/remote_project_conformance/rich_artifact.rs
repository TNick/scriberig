//! Compares mixed rich-artifact reads through embedded and remote public APIs.

use std::{fs, sync::Arc};

use rusqlite::Connection;
use scriberig_app::{
    ApplicationContext, ApplicationService as AppService, Capability, CapabilityAction,
    CapabilityConstraints, CapabilityContext, CapabilityScope, StoreInspectionBackend,
};
use scriberig_client::Client;
use scriberig_core::EntityId;
use scriberig_project::ProjectService;
use scriberig_server::InProcessTransport;
use scriberig_session_sqlite::SqliteSessionStore;
use scriberig_store_sqlite::SqliteStore;

use super::application;

#[path = "rich_artifact/artifact_inspection.rs"]
mod artifact_inspection;
#[path = "rich_artifact/cad.rs"]
mod cad;
#[path = "rich_artifact/document.rs"]
mod document;
#[path = "rich_artifact/geo.rs"]
mod geo;
#[path = "rich_artifact/qgis.rs"]
mod qgis;
#[path = "rich_artifact/spreadsheet_search.rs"]
mod spreadsheet_search;

type EmbeddedApplication = AppService<StoreInspectionBackend<Arc<SqliteStore>>>;
type RemoteApplicationClient = Client<InProcessTransport<super::TestApplication>>;

/// Confirms mixed-project XLSX, DOCX, CAD, `GeoPackage`, and QGIS reads match across app and client.
#[test]
fn mixed_rich_artifact_reads_match_embedded_and_remote_contracts() {
    let root = std::env::temp_dir().join(format!("scriberig-remote-rich-{}", EntityId::new()));
    copy_mixed_fixture(&root);
    Connection::open(root.join("parcels.gpkg"))
        .expect("open copied GeoPackage")
        .execute_batch("CREATE TRIGGER parcel_audit AFTER UPDATE ON parcels BEGIN SELECT 1; END;")
        .expect("add preservation-sensitive GeoPackage trigger");
    let database = root.with_extension("scriberig.sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("project database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let project = projects
        .scan(None, &root)
        .expect("scan copied project")
        .project
        .id;
    let imported = projects.import(project).expect("import mixed project");
    let entities = store.project_entities(project).expect("project entities");
    let drawing = entities
        .iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("semantic CAD drawing")
        .entity;
    let layer = entities
        .iter()
        .find(|entity| entity.kind == "geo.layer" && entity.relative_path == "parcels.gpkg")
        .expect("semantic GeoPackage layer")
        .entity;
    let workbook = entities
        .iter()
        .find(|entity| entity.kind == "spreadsheet.workbook")
        .expect("semantic spreadsheet workbook")
        .entity;
    let document = entities
        .iter()
        .find(|entity| entity.kind == "document.root")
        .expect("semantic rich document")
        .entity;
    let context = project_inspect_context(project, imported.snapshot);
    let embedded = AppService::new(StoreInspectionBackend::new(Arc::clone(&store)))
        .with_project_service(projects.clone());
    let sessions = Arc::new(SqliteSessionStore::open_in_memory().expect("session store"));
    let remote = Client::new(
        InProcessTransport::new(
            application(Arc::clone(&store), sessions).with_project_service(projects.clone()),
        )
        .with_capabilities(context.capabilities.clone()),
    );

    artifact_inspection::assert_artifact_fidelity_matches(&embedded, &remote, &context, project);

    let worksheet = spreadsheet_search::assert_workbook_and_find(
        &embedded,
        &remote,
        &context,
        project,
        imported.snapshot,
        workbook,
    );
    spreadsheet_search::assert_cell_page(
        &embedded,
        &remote,
        &context,
        project,
        imported.snapshot,
        worksheet,
    );
    document::assert_document_reads(
        &embedded,
        &remote,
        &context,
        project,
        imported.snapshot,
        document,
    );
    cad::assert_cad_reads(
        &embedded,
        &remote,
        &context,
        project,
        imported.snapshot,
        drawing,
    );
    geo::assert_geo_reads(
        &embedded,
        &remote,
        &context,
        project,
        imported.snapshot,
        layer,
    );
    qgis::assert_qgis_read(
        &embedded,
        &remote,
        &context,
        project,
        imported.snapshot,
        layer,
    );
    drop(remote);
    drop(embedded);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove copied project");
}

/// Copies the checked-in mixed artifact fixture into a disposable project root.
///
/// - `root`: Destination directory for the copied fixture.
fn copy_mixed_fixture(root: &std::path::Path) {
    let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/rich-project");
    fs::create_dir_all(root).expect("create copied project");
    for entry in fs::read_dir(fixture).expect("read checked-in fixture") {
        let entry = entry.expect("fixture entry");
        if entry.file_type().expect("fixture file type").is_file()
            && entry.file_name() != "README.md"
        {
            fs::copy(entry.path(), root.join(entry.file_name())).expect("copy fixture member");
        }
    }
}

/// Creates exact-snapshot Inspect authority for one project.
///
/// - `project`: Project to authorize.
/// - `snapshot`: Exact imported snapshot.
///
/// # Returns
/// Application context restricted to Inspect for the selected project.
fn project_inspect_context(
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ApplicationContext {
    ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: std::collections::BTreeSet::from([CapabilityAction::Inspect]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    }
}
