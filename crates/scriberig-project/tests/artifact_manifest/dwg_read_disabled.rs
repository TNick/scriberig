//! Verifies providers without read capability are not invoked for DWG imports.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::dwg_provider::FakeDwgProvider;
use scriberig_project::{ArtifactCapability, ProjectService};
use scriberig_store_sqlite::SqliteStore;

/// Keeps exact DWG bytes without invoking a provider that denied read capability.
#[test]
fn dwg_provider_without_read_capability_is_not_invoked() {
    let root = crate::temporary_project();
    fs::write(root.join("unreadable.dwg"), b"opaque DWG source evidence")
        .expect("write source fixture");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let import_calls = Arc::new(AtomicUsize::new(0));
    let registry = scriberig_domain_cad::CadProviderRegistry::new()
        .register(Arc::new(FakeDwgProvider {
            provider_id: "fixture.dwg",
            provider_version: "2.4",
            import_identity_override: None,
            native_handle: "native-handle-77",
            change_line_geometry: false,
            read: false,
            unmapped_object_count: 0,
            import_calls: Arc::clone(&import_calls),
        }))
        .expect("register non-reading provider");
    let service = ProjectService::new(Arc::clone(&store)).with_cad_provider_registry(registry);
    let scan = service.scan(None, &root).expect("scan source");
    service
        .import(scan.project.id)
        .expect("preserve source evidence");

    assert_eq!(import_calls.load(Ordering::SeqCst), 0);
    assert!(
        store
            .project_entities(scan.project.id)
            .expect("project semantic ownership")
            .iter()
            .all(|entity| entity.kind != "cad.drawing")
    );
    let artifact = service
        .artifacts(scan.project.id)
        .expect("provider-aware artifact fidelity")
        .into_iter()
        .next()
        .expect("DWG artifact");
    assert_eq!(
        artifact.fidelity.capability,
        ArtifactCapability::InspectOnly
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove source fixture");
}
