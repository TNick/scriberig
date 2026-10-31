//! Verifies provider import provenance matches its registered descriptor.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use super::dwg_provider::FakeDwgProvider;
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Rejects provider-controlled import identity that differs from its registration.
#[test]
fn dwg_import_identity_must_match_registered_provider() {
    let root = crate::temporary_project();
    fs::write(root.join("model.dwg"), b"identity mismatch fixture").expect("write source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let registry = scriberig_domain_cad::CadProviderRegistry::new()
        .register(Arc::new(FakeDwgProvider {
            provider_id: "registered.provider",
            provider_version: "1.0",
            import_identity_override: Some(("unregistered.provider", "9.9")),
            native_handle: "native-handle-77",
            change_line_geometry: false,
            read: true,
            unmapped_object_count: 0,
            import_calls: Arc::new(AtomicUsize::new(0)),
        }))
        .expect("register provider");
    let service = ProjectService::new(Arc::clone(&store)).with_cad_provider_registry(registry);
    let scan = service.scan(None, &root).expect("scan source");

    let error = service
        .import(scan.project.id)
        .expect_err("mismatched provider provenance must fail closed");
    assert_eq!(error.code, "import_diagnostics");
    assert!(error.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "dwg_provider_import_invalid"
            && diagnostic
                .message
                .contains("exceeded its advertised identity")
    }));
    assert!(
        store
            .project_entities(scan.project.id)
            .expect("source ownership remains durable")
            .iter()
            .all(|entity| entity.kind != "cad.drawing")
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove source fixture");
}
