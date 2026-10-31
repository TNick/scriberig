//! Verifies authorized application artifact reads use bounded persisted summaries.

use std::collections::BTreeSet;
use std::fs;
use std::io::{Cursor, Write};
use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationService, Capability, CapabilityAction,
    CapabilityConstraints, CapabilityContext, CapabilityScope,
};
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;
use zip::write::SimpleFileOptions;

/// Provides an exact project-scoped artifact read grant.
///
/// # Parameters
/// - `project`: Stable owning project identity.
///
/// # Returns
/// Application context carrying the project's Inspect grant.
fn authorized_context(project: scriberig_core::ProjectId) -> ApplicationContext {
    ApplicationContext {
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{project}")),
            actions: BTreeSet::from([CapabilityAction::Inspect]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    }
}

/// Reads bounded manifests from the durable scan and rejects unauthorized reads.
#[test]
fn application_artifact_reads_are_project_scoped_and_bounded() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-artifacts-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    fs::write(root.join("one.txt"), b"one").expect("write first file");
    fs::write(root.join("two.txt"), b"two").expect("write second file");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let service = ApplicationService::new(()).with_project_service(projects);

    assert!(matches!(
        service.inspect_artifacts(&ApplicationContext::default(), scan.project.id, 0, 1),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    assert!(matches!(
        service.inspect_artifact_members(
            &ApplicationContext::default(),
            scan.project.id,
            scriberig_core::EntityId::new(),
            0,
            1,
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    let context = authorized_context(scan.project.id);
    let first = service
        .inspect_artifacts(&context, scan.project.id, 0, 1)
        .expect("first bounded page");
    assert_eq!(first.total, 2);
    assert_eq!(first.artifacts.len(), 1);
    assert_eq!(first.next_offset, Some(1));
    assert_eq!(
        first.synchronization[0].status,
        scriberig_project::ArtifactSynchronizationState::InSync
    );
    assert_eq!(
        first.synchronization[0].baseline_digest,
        first.synchronization[0].observed_digest.as_deref().unwrap()
    );
    fs::write(root.join("one.txt"), b"external edit").expect("change live source");
    let changed = service
        .inspect_artifacts(&context, scan.project.id, 0, 1)
        .expect("inspect modified artifact");
    assert_eq!(
        changed.synchronization[0].status,
        scriberig_project::ArtifactSynchronizationState::Modified
    );
    assert_ne!(
        changed.synchronization[0].baseline_digest,
        changed.synchronization[0]
            .observed_digest
            .as_deref()
            .unwrap()
    );
    fs::remove_file(root.join("one.txt")).expect("remove live source");
    let missing = service
        .inspect_artifacts(&context, scan.project.id, 0, 1)
        .expect("inspect missing artifact");
    assert_eq!(
        missing.synchronization[0].status,
        scriberig_project::ArtifactSynchronizationState::Missing
    );
    assert!(missing.synchronization[0].observed_digest.is_none());
    let second = service
        .inspect_artifacts(&context, scan.project.id, 1, 1)
        .expect("second bounded page");
    assert_eq!(second.artifacts.len(), 1);
    assert_ne!(first.artifacts[0].id, second.artifacts[0].id);
    let members = service
        .inspect_artifact_members(
            &context,
            scan.project.id,
            first.artifacts[0].id.entity(),
            0,
            1,
        )
        .expect("authorized member page");
    assert_eq!(members.total, 1);
    assert_eq!(members.members.len(), 1);
    assert_eq!(members.members[0].project_path.as_deref(), Some("one.txt"));
    assert_eq!(members.next_offset, None);
    assert!(matches!(
        service.inspect_artifacts(&context, scan.project.id, 0, 101),
        Err(ApplicationError::InvalidRequest { .. })
    ));

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Exposes adapter fidelity and diagnostics through shared application semantics.
#[test]
fn application_artifact_page_includes_xlsx_read_only_diagnostic() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-xlsx-fidelity-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    fs::write(root.join("broken.xlsx"), b"not an XLSX package").expect("write invalid package");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let service = ApplicationService::new(()).with_project_service(projects);
    let page = service
        .inspect_artifacts(&authorized_context(scan.project.id), scan.project.id, 0, 10)
        .expect("authorized artifact page");
    assert_eq!(
        page.artifacts[0].fidelity.capability,
        scriberig_project::ArtifactCapability::InspectOnly
    );
    assert!(page.artifacts[0].fidelity.diagnostic.is_some());
    assert_eq!(page.artifacts[0].source.adapter, "xlsx");
    assert!(scan.project.current_snapshot.is_none());
    assert!(
        !store
            .project_entities(scan.project.id)
            .expect("project semantic entities")
            .iter()
            .any(|entity| entity.kind == "spreadsheet.workbook")
    );
    let propose_context = ApplicationContext {
        snapshot: Some(scriberig_core::SnapshotId::new([88; 32])),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Custom(format!("project:{}", scan.project.id)),
            actions: BTreeSet::from([CapabilityAction::Propose]),
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        ..ApplicationContext::default()
    };
    assert!(matches!(
        service.propose_spreadsheet_semantic_cell_edit(
            &propose_context,
            scan.project.id,
            scriberig_core::EntityId::new(),
            &scriberig_domain_spreadsheet::CellValue::Text("unsafe edit".into()),
        ),
        Err(ApplicationError::InvalidRequest { .. })
    ));

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Exposes the DOCX adapter's probed edit and preservation capability to Inspect clients.
#[test]
fn application_artifact_page_reports_docx_targeted_edit_capability() {
    let root = std::env::temp_dir().join(format!(
        "scriberig-app-docx-fidelity-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create project");
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file("word/document.xml", SimpleFileOptions::default())
        .expect("document entry");
    archive
        .write_all(br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Keep</w:t></w:r></w:p></w:body></w:document>"#)
        .expect("document XML");
    fs::write(
        root.join("report.docx"),
        archive.finish().expect("finish package").into_inner(),
    )
    .expect("write DOCX package");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let projects = ProjectService::new(Arc::clone(&store));
    let scan = projects.scan(None, &root).expect("scan project");
    let service = ApplicationService::new(()).with_project_service(projects);
    let page = service
        .inspect_artifacts(&authorized_context(scan.project.id), scan.project.id, 0, 10)
        .expect("authorized artifact page");
    let document = page.artifacts.first().expect("DOCX artifact");
    assert_eq!(
        document.fidelity.capability,
        scriberig_project::ArtifactCapability::EditableWithOpaquePreservation
    );
    assert_eq!(
        document.fidelity.opaque_preservation,
        scriberig_project::OpaquePreservation::Preserved
    );
    assert!(
        document
            .fidelity
            .diagnostic
            .as_deref()
            .unwrap()
            .contains("complex structures remain read-only")
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Denies `GeoPackage` reads without an exact project-scoped Inspect grant.
#[test]
fn geopackage_feature_reads_require_snapshot_and_inspect_authority() {
    let service = ApplicationService::new(());
    let project = scriberig_core::ProjectId::from_bytes([21; 16]);
    let snapshot = scriberig_core::SnapshotId::new([22; 32]);
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        ..ApplicationContext::default()
    };
    assert!(matches!(
        service.read_geopackage_features(
            &context,
            project,
            "parcels.gpkg",
            scriberig_core::EntityId::from_bytes([23; 16]),
            0,
            10,
        ),
        Err(ApplicationError::PermissionDenied { .. })
    ));
    assert!(matches!(
        service.inspect_geopackage(&context, project, "parcels.gpkg", 0, 10),
        Err(ApplicationError::PermissionDenied { .. })
    ));
}
