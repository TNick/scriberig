//! Verifies provider changes cannot silently replace persisted CAD identities.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use super::dwg_provider::FakeDwgProvider;
use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Blocks provider lineage changes until explicit semantic reconciliation exists.
#[test]
fn dwg_provider_switch_reconciles_semantic_entities_across_native_handles() {
    let root = crate::temporary_project();
    fs::write(root.join("model.dwg"), b"stable DWG source evidence").expect("write source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let first_registry = registry("provider.first", "1.0", "native-A", false);
    let first_service =
        ProjectService::new(Arc::clone(&store)).with_cad_provider_registry(first_registry);
    let scan = first_service.scan(None, &root).expect("scan source");
    let first_import = first_service
        .import(scan.project.id)
        .expect("first provider import");
    let mut original_entity_ids = store
        .project_entities(scan.project.id)
        .expect("first provider semantic entities")
        .into_iter()
        .filter(|entity| entity.kind == "cad.entity")
        .map(|entity| entity.entity)
        .collect::<Vec<_>>();
    original_entity_ids.sort();

    let switched = ProjectService::new(Arc::clone(&store)).with_cad_provider_registry(registry(
        "provider.second",
        "1.0",
        "native-B",
        false,
    ));
    let second_import = switched
        .import(scan.project.id)
        .expect("unambiguous semantic matching reconciles provider change");
    let mut reconciled_entity_ids = store
        .project_entities(scan.project.id)
        .expect("second provider semantic entities")
        .into_iter()
        .filter(|entity| entity.kind == "cad.entity")
        .map(|entity| entity.entity)
        .collect::<Vec<_>>();
    reconciled_entity_ids.sort();
    assert_eq!(reconciled_entity_ids, original_entity_ids);
    assert_ne!(second_import.snapshot, first_import.snapshot);
    let drawing_id = store
        .project_entities(scan.project.id)
        .expect("drawing entity")
        .into_iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("CAD drawing")
        .entity;
    let page = switched
        .cad_entities(scan.project.id, second_import.snapshot, drawing_id, 0, 10)
        .expect("read reconciled entities");
    let reconciled_line = page
        .entities
        .iter()
        .find(|entity| {
            matches!(
                entity.geometry,
                scriberig_domain_cad::CadEntity::Line { .. }
            )
        })
        .expect("reconciled line");
    assert!(original_entity_ids.contains(&reconciled_line.id));
    assert_eq!(
        reconciled_line.geometry,
        scriberig_domain_cad::CadEntity::Line {
            start: scriberig_domain_cad::Coordinate::new("0", "0", "0"),
            end: scriberig_domain_cad::Coordinate::new("10", "0", "0"),
        }
    );

    drop(switched);
    drop(first_service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove source fixture");
}

/// Rejects provider changes that lack exact semantic continuity evidence.
#[test]
fn dwg_provider_switch_with_changed_geometry_requires_explicit_reconciliation() {
    let root = crate::temporary_project();
    fs::write(root.join("model.dwg"), b"changed geometry fixture").expect("write source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open database"));
    let first = ProjectService::new(Arc::clone(&store)).with_cad_provider_registry(registry(
        "provider.first",
        "1.0",
        "native-A",
        false,
    ));
    let scan = first.scan(None, &root).expect("scan source");
    let initial = first
        .import(scan.project.id)
        .expect("initial provider import");
    let switched = ProjectService::new(Arc::clone(&store)).with_cad_provider_registry(registry(
        "provider.second",
        "1.0",
        "native-B",
        true,
    ));

    let error = switched
        .import(scan.project.id)
        .expect_err("changed geometry needs stronger reconciliation evidence");
    assert_eq!(error.code, "import_diagnostics");
    assert!(
        error
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "dwg_provider_import_invalid" })
    );
    assert_eq!(
        first
            .workspace(scan.project.id)
            .expect("current workspace")
            .snapshot,
        initial.snapshot
    );

    drop(switched);
    drop(first);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove source fixture");
}

/// Creates a registry containing one named fake provider.
///
/// # Parameters
/// - `id`: Safe provider identity.
/// - `version`: Safe provider version.
/// - `native_handle`: Provider-native identifier returned for the fixture entity.
/// - `change_line_geometry`: Whether the fake provider changes line geometry.
///
/// # Returns
/// A validated registry for the requested DWG provider.
fn registry(
    id: &'static str,
    version: &'static str,
    native_handle: &'static str,
    change_line_geometry: bool,
) -> scriberig_domain_cad::CadProviderRegistry {
    scriberig_domain_cad::CadProviderRegistry::new()
        .register(Arc::new(FakeDwgProvider {
            provider_id: id,
            provider_version: version,
            import_identity_override: None,
            native_handle,
            change_line_geometry,
            read: true,
            unmapped_object_count: 0,
            import_calls: Arc::new(AtomicUsize::new(0)),
        }))
        .expect("register fake provider")
}
