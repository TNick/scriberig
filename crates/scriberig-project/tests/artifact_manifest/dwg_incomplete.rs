//! Verifies unsupported provider objects make imported CAD semantics incomplete.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use super::dwg_provider::FakeDwgProvider;
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Prevents provider-omitted source objects from appearing fully mapped.
#[test]
fn dwg_provider_unmapped_objects_mark_shared_drawing_incomplete() {
    let root = crate::temporary_project();
    fs::write(root.join("partial.dwg"), b"partial DWG source evidence")
        .expect("write source fixture");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let registry = scriberig_domain_cad::CadProviderRegistry::new()
        .register(Arc::new(FakeDwgProvider {
            provider_id: "fixture.dwg",
            provider_version: "2.4",
            import_identity_override: None,
            native_handle: "native-handle-77",
            change_line_geometry: false,
            read: true,
            unmapped_object_count: 2,
            import_calls: Arc::new(AtomicUsize::new(0)),
        }))
        .expect("register read-only provider");
    let service = ProjectService::new(Arc::clone(&store)).with_cad_provider_registry(registry);
    let scan = service.scan(None, &root).expect("scan source");
    let imported = service.import(scan.project.id).expect("provider import");
    let drawing = store
        .project_entities(scan.project.id)
        .expect("project semantic ownership")
        .into_iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("shared CAD drawing");
    let view = service
        .cad_drawing(scan.project.id, imported.snapshot, drawing.entity)
        .expect("inspect incomplete imported drawing");

    assert!(!view.complete);
    assert!(
        view.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("2 source objects"))
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove source fixture");
}
