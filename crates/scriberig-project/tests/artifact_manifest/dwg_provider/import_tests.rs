//! Verifies provider-imported DWG data enters shared semantic project APIs.

use std::fs;
use std::sync::Arc;

use scriberig_project::{ArtifactCapability, ProjectService};
use scriberig_store::{RevisionStore, SnapshotStore};
use scriberig_store_sqlite::SqliteStore;

use super::*;

/// Imports provider output and verifies detached handles and read-only fidelity.
#[test]
fn configured_dwg_provider_imports_shared_semantics_and_detached_provenance() {
    let root = super::super::temporary_project();
    let source = b"deterministic DWG source evidence";
    fs::write(root.join("model.dwg"), source).expect("write source fixture");
    let (service, store, project, snapshot) = imported_fixture(&root);
    let evidence = assert_imported_drawing(&service, &store, project, snapshot);
    assert_detached_provenance(&store, snapshot, &evidence);
    assert_source_and_fidelity(&service, &store, project, source);
    drop(service);
    drop(store);
    fs::remove_file(root.with_extension("sqlite")).expect("remove database");
    fs::remove_dir_all(root).expect("remove source fixture");
}

/// Imports the fixture through the configured fake DWG provider.
///
/// # Parameters
/// - `root`: Temporary project directory containing the DWG source.
///
/// # Returns
/// Project service, store, exact project ID, and imported snapshot.
///
/// # Panics
/// Panics when database creation, provider registration, scan, or import fails.
fn imported_fixture(
    root: &std::path::Path,
) -> (
    ProjectService,
    Arc<SqliteStore>,
    scriberig_core::ProjectId,
    scriberig_core::SnapshotId,
) {
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
            unmapped_object_count: 0,
            import_calls: Arc::new(AtomicUsize::new(0)),
        }))
        .expect("register read-only provider");
    let service = ProjectService::new(Arc::clone(&store)).with_cad_provider_registry(registry);
    let scan = service.scan(None, root).expect("scan source");
    let imported = service.import(scan.project.id).expect("provider import");
    (service, store, scan.project.id, imported.snapshot)
}

/// Confirms provider-imported data uses shared CAD inspection APIs.
///
/// # Parameters
/// - `service`: Project service with the configured provider.
/// - `store`: Store containing the imported project.
/// - `project`: Exact imported project.
/// - `snapshot`: Exact provider-imported snapshot.
///
/// # Returns
/// Evidence containing the shared drawing and first semantic entity IDs.
///
/// # Panics
/// Panics when shared semantic ownership or CAD read APIs differ from expectations.
fn assert_imported_drawing(
    service: &ProjectService,
    store: &SqliteStore,
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> ImportedDrawingEvidence {
    let entities = store
        .project_entities(project)
        .expect("project semantic ownership");
    let drawing = entities
        .iter()
        .find(|entity| entity.kind == "cad.drawing")
        .expect("shared CAD drawing");
    let drawing_view = service
        .cad_drawing(project, snapshot, drawing.entity)
        .expect("inspect provider-imported drawing through shared CAD API");
    assert_eq!(drawing_view.layers[0].name, "Roads");
    assert_eq!(drawing_view.entity_count, 2);
    assert!(drawing_view.complete);
    for kind in [
        "cad.block_definition",
        "cad.layout",
        "cad.viewport",
        "cad.external_reference",
    ] {
        assert!(
            entities.iter().any(|entity| entity.kind == kind),
            "provider import should persist {kind} semantics"
        );
    }
    let entity_page = service
        .cad_entities(project, snapshot, drawing.entity, 0, 1)
        .expect("list provider-imported entities through shared CAD API");
    assert_eq!(entity_page.total, 2);
    assert_eq!(entity_page.entities[0].layer, drawing_view.layers[0].id);
    let text_matches = service
        .find_cad_text(project, snapshot, drawing.entity, "north gate", 0, 10)
        .expect("find provider-imported annotations through shared CAD API");
    assert_eq!(text_matches.total, 1);
    assert_eq!(text_matches.matches[0].text, "North Gate");
    let semantic_entity = entities
        .iter()
        .find(|entity| entity.kind == "cad.entity")
        .expect("shared CAD entity");
    assert_ne!(
        semantic_entity.entity,
        scriberig_core::EntityId::from_bytes([3; 16]),
        "provider-created IDs must be remapped by the project adapter"
    );
    let summary: serde_json::Value = serde_json::from_slice(
        store
            .resolve(snapshot, drawing.entity)
            .expect("resolve drawing")
            .and_then(|revision| store.get(revision).expect("load drawing revision"))
            .expect("stored drawing revision")
            .payload(),
    )
    .expect("drawing summary");
    assert_eq!(summary["provider_id"], "fixture.dwg");
    assert_eq!(summary["provider_version"], "2.4");
    assert_eq!(
        summary["entity_identifiers"][0]["value"],
        "native-handle-77"
    );
    assert_eq!(summary["entities"].as_array().expect("entity IDs").len(), 2);
    let drawing_entity_ids: Vec<scriberig_core::EntityId> =
        serde_json::from_value(summary["entities"].clone()).expect("semantic entity IDs");
    assert!(drawing_entity_ids.contains(&semantic_entity.entity));
    ImportedDrawingEvidence {
        semantic_entity: semantic_entity.entity,
        summary,
    }
}

/// Asserts provider IDs remain detached from semantic entity payloads.
///
/// # Parameters
/// - `store`: Store containing the imported DWG project.
/// - `snapshot`: Exact imported snapshot.
/// - `evidence`: Verified drawing and semantic entity provenance.
///
/// # Panics
/// Panics when native provider handles leak into semantic identity or payload.
fn assert_detached_provenance(
    store: &SqliteStore,
    snapshot: scriberig_core::SnapshotId,
    evidence: &ImportedDrawingEvidence,
) {
    let entity = evidence.semantic_entity;
    let drawing_entity_ids: Vec<scriberig_core::EntityId> =
        serde_json::from_value(evidence.summary["entities"].clone()).expect("semantic entity IDs");
    assert!(drawing_entity_ids.contains(&entity));
    assert_eq!(evidence.summary["provider_id"], "fixture.dwg");
    let entity_revision = store
        .resolve(snapshot, entity)
        .expect("resolve drawing")
        .and_then(|revision| store.get(revision).expect("load CAD entity revision"))
        .expect("stored CAD entity revision");
    let entity_json: serde_json::Value =
        serde_json::from_slice(entity_revision.payload()).expect("semantic entity JSON");
    let serialized_entity =
        String::from_utf8(entity_revision.payload().to_vec()).expect("entity JSON UTF-8");
    assert!(!serialized_entity.contains("native-handle-77"));
    assert!(entity_json.get("provider_id").is_none());
}

/// Confirms exact source evidence and provider-aware artifact fidelity.
///
/// # Parameters
/// - `service`: Project service with the configured provider.
/// - `store`: Store containing the imported project.
/// - `project`: Exact imported project.
/// - `source`: Exact input bytes used to compute the source digest.
///
/// # Panics
/// Panics when source provenance or read-only fidelity differs from expectations.
fn assert_source_and_fidelity(
    service: &ProjectService,
    store: &SqliteStore,
    project: scriberig_core::ProjectId,
    source: &[u8],
) {
    let descriptor = store
        .project_file_descriptors(project)
        .expect("exact source descriptors")
        .into_iter()
        .find(|file| file.relative_path == "model.dwg")
        .expect("source evidence");
    assert_eq!(descriptor.digest, *blake3::hash(source).as_bytes());
    let artifact = service
        .artifacts(project)
        .expect("provider-aware artifact fidelity")
        .into_iter()
        .next()
        .expect("DWG artifact");
    assert_eq!(
        artifact.fidelity.capability,
        ArtifactCapability::ReadOnlyDerived
    );
    assert!(
        artifact
            .fidelity
            .diagnostic
            .unwrap()
            .contains("not writing")
    );
}

/// Carries IDs and the verified provider drawing summary between assertions.
#[derive(Debug)]
struct ImportedDrawingEvidence {
    /// First shared semantic entity ID.
    semantic_entity: scriberig_core::EntityId,
    /// Persisted summary containing provider provenance.
    summary: serde_json::Value,
}
