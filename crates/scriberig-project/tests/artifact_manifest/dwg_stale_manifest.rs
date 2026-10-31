//! Verifies stale DWG scans cannot replace the imported source baseline.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use super::dwg_provider::FakeDwgProvider;
use scriberig_project::ProjectService;
use scriberig_store::SnapshotStore;
use scriberig_store_sqlite::SqliteStore;

/// Rejects provider import when the exact source scan was superseded.
#[test]
fn stale_dwg_manifest_cannot_replace_imported_source_baseline() {
    let root = crate::temporary_project();
    let source_path = root.join("model.dwg");
    let original = b"initial exact DWG bytes";
    fs::write(&source_path, original).expect("write source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let service = ProjectService::new(Arc::clone(&store)).with_cad_provider_registry(
        scriberig_domain_cad::CadProviderRegistry::new()
            .register(Arc::new(FakeDwgProvider {
                provider_id: "fixture.dwg",
                provider_version: "1.0",
                import_identity_override: None,
                native_handle: "native-baseline",
                change_line_geometry: false,
                read: true,
                unmapped_object_count: 0,
                import_calls: Arc::new(AtomicUsize::new(0)),
            }))
            .expect("register provider"),
    );
    let scan = service.scan(None, &root).expect("scan exact source");
    let summary = service
        .manifest_summary(scan.project.id)
        .expect("review initial manifest");
    let initial = service
        .import_manifest(scan.project.id, &summary.manifest_id)
        .expect("import reviewed source");
    let drawing = store
        .project_entities(scan.project.id)
        .expect("drawing ownership")
        .into_iter()
        .find(|record| record.kind == "cad.drawing")
        .expect("provider drawing");

    fs::write(&source_path, b"changed external DWG bytes").expect("change source externally");
    let changed_scan = service.scan(None, &root).expect("persist changed scan");
    let error = service
        .import_manifest(scan.project.id, &summary.manifest_id)
        .expect_err("old exact manifest must be rejected");
    assert_eq!(error.code, "manifest_changed");
    let workspace = service
        .workspace(scan.project.id)
        .expect("active workspace after rejection");
    assert_eq!(workspace.snapshot, initial.snapshot);
    let source = store
        .project_file_descriptors(scan.project.id)
        .expect("persisted descriptors")
        .into_iter()
        .find(|file| file.relative_path == "model.dwg")
        .expect("DWG descriptor");
    assert_eq!(
        source.digest,
        *blake3::hash(b"changed external DWG bytes").as_bytes()
    );
    assert_ne!(source.digest, *blake3::hash(original).as_bytes());
    assert_eq!(
        changed_scan.project.current_snapshot,
        Some(initial.snapshot)
    );
    assert!(
        store
            .resolve(initial.snapshot, drawing.entity)
            .expect("active drawing mapping")
            .is_some()
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove source fixture");
}
