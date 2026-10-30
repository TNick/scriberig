//! Verifies deterministic artifact grouping over exact persisted scan evidence.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_domain_cad::{
    CadFormat, CadProviderFeature, CadProviderFormatCapability, CadProviderOpaquePreservation,
};
use scriberig_project::{
    ArtifactCapability, ArtifactId, ArtifactKind, OpaquePreservation, ProjectFileRename,
    ProjectService, ScanOptions, cad_provider_artifact_fidelity,
};
use scriberig_store::SnapshotStore;

#[path = "artifact_manifest/container_members.rs"]
mod container_members;
#[path = "artifact_manifest/dwg_incomplete.rs"]
mod dwg_incomplete;
#[path = "artifact_manifest/dwg_provider.rs"]
mod dwg_provider;
#[path = "artifact_manifest/dwg_provider_identity.rs"]
mod dwg_provider_identity;
#[path = "artifact_manifest/dwg_provider_switch.rs"]
mod dwg_provider_switch;
#[path = "artifact_manifest/dwg_read_disabled.rs"]
mod dwg_read_disabled;
#[path = "artifact_manifest/dwg_stale_manifest.rs"]
mod dwg_stale_manifest;
#[path = "artifact_manifest/fidelity.rs"]
mod fidelity;
#[path = "artifact_manifest/rename.rs"]
mod rename;
use scriberig_store_sqlite::SqliteStore;

/// Creates a unique temporary project directory.
///
/// # Returns
/// A fresh empty directory under the operating system temporary path.
fn temporary_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-artifacts-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create fixture");
    root
}

/// Creates one valid point Shapefile record for grouping/import fixtures.
///
/// # Returns
/// Exact `.shp` bytes with one origin point.
fn shapefile_points() -> Vec<u8> {
    let mut bytes = vec![0; 100];
    bytes[0..4].copy_from_slice(&9994_i32.to_be_bytes());
    bytes[24..28].copy_from_slice(&64_i32.to_be_bytes());
    bytes[28..32].copy_from_slice(&1000_i32.to_le_bytes());
    bytes[32..36].copy_from_slice(&1_i32.to_le_bytes());
    bytes.extend_from_slice(&1_i32.to_be_bytes());
    bytes.extend_from_slice(&10_i32.to_be_bytes());
    bytes.extend_from_slice(&1_i32.to_le_bytes());
    bytes.extend_from_slice(&0.0_f64.to_le_bytes());
    bytes.extend_from_slice(&0.0_f64.to_le_bytes());
    bytes
}

/// Creates a valid one-row dBase file for grouping/import fixtures.
///
/// # Returns
/// Exact `.dbf` bytes with one numeric identifier field.
fn shapefile_attributes() -> Vec<u8> {
    let mut bytes = vec![0; 65];
    bytes[0] = 3;
    bytes[4..8].copy_from_slice(&1_u32.to_le_bytes());
    bytes[8..10].copy_from_slice(&65_u16.to_le_bytes());
    bytes[10..12].copy_from_slice(&6_u16.to_le_bytes());
    bytes[32..43].copy_from_slice(b"road_id\0\0\0\0");
    bytes[43] = b'N';
    bytes[48] = 5;
    bytes[64] = 0x0d;
    bytes.extend_from_slice(b"    42");
    bytes
}

/// Maps an optional DWG provider's independent capabilities into shared fidelity.
#[test]
fn dwg_provider_fidelity_requires_validation_and_accounts_for_opaque_content() {
    let read_only = CadProviderFormatCapability {
        format: CadFormat::Dwg,
        read: true,
        write: false,
        validate: false,
        opaque_preservation: CadProviderOpaquePreservation::Unproven,
        supported_features: vec![CadProviderFeature::Line],
    };
    let fidelity = cad_provider_artifact_fidelity(&read_only);
    assert_eq!(fidelity.capability, ArtifactCapability::ReadOnlyDerived);
    assert_eq!(fidelity.opaque_preservation, OpaquePreservation::Unproven);

    let mut writable = read_only;
    writable.write = true;
    writable.validate = true;
    writable.opaque_preservation = CadProviderOpaquePreservation::Preserved;
    let fidelity = cad_provider_artifact_fidelity(&writable);
    assert_eq!(
        fidelity.capability,
        ArtifactCapability::EditableWithOpaquePreservation
    );
    assert_eq!(fidelity.opaque_preservation, OpaquePreservation::Preserved);

    writable.validate = false;
    let fidelity = cad_provider_artifact_fidelity(&writable);
    assert_eq!(
        fidelity.capability,
        ArtifactCapability::RestrictedExternalProviderEdit
    );
    assert!(fidelity.diagnostic.is_some());
}

/// Keeps native DWG bytes visible as a read-only artifact when no provider is configured.
#[test]
fn dwg_onboarding_reports_unavailable_provider_and_retains_source_evidence() {
    let root = temporary_project();
    let source = b"DWG source evidence";
    fs::write(root.join("model.dwg"), source).expect("write DWG evidence");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan DWG");
    let artifacts = service
        .artifacts(scan.project.id)
        .expect("artifact summary");
    let dwg = artifacts.first().expect("DWG artifact");
    assert_eq!(dwg.source.adapter, "dwg");
    assert_eq!(dwg.members.len(), 1);
    assert_eq!(dwg.members[0].digest, *blake3::hash(source).as_bytes());
    assert_eq!(dwg.fidelity.capability, ArtifactCapability::ReadOnlyDerived);
    assert!(
        dwg.fidelity
            .diagnostic
            .as_deref()
            .unwrap()
            .contains("No optional DWG provider")
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Imports an unknown binary through the fallback without blocking the project.
#[test]
fn unknown_binary_fallback_preserves_source_and_imports_metadata() {
    let root = temporary_project();
    let source = [0, 159, 146, 150, 255, 0, 1, 2];
    fs::write(root.join("opaque.payload"), source).expect("write unknown binary");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan unknown binary");
    assert_eq!(scan.files.len(), 1);
    assert_eq!(scan.files[0].adapter, "binary");
    assert_eq!(scan.files[0].status, "ready");
    assert_eq!(scan.files[0].bytes, source);

    service
        .import(scan.project.id)
        .expect("import unknown binary");
    let artifact = service
        .artifacts(scan.project.id)
        .expect("inspect unknown artifact")
        .into_iter()
        .next()
        .expect("one opaque artifact");
    assert_eq!(
        artifact.fidelity.capability,
        ArtifactCapability::ReadOnlyDerived
    );
    assert_eq!(
        artifact.fidelity.opaque_preservation,
        OpaquePreservation::Unproven
    );
    assert!(artifact.fidelity.diagnostic.is_some());
    let descriptor = store
        .project_file_descriptors(scan.project.id)
        .expect("persisted source evidence")
        .into_iter()
        .find(|file| file.relative_path == "opaque.payload")
        .expect("binary evidence");
    assert_eq!(descriptor.digest, *blake3::hash(&source).as_bytes());
    assert!(
        store
            .project_entities(scan.project.id)
            .expect("semantic entities")
            .iter()
            .any(|entity| entity.kind == "binary.document")
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Groups persisted Shapefile members and keeps similarly named text separate.
#[test]
fn groups_compound_members_from_persisted_scan_after_filesystem_changes() {
    let root = temporary_project();
    let shape = shapefile_points();
    let attributes = shapefile_attributes();
    for (path, bytes) in [
        ("roads.shp", shape.as_slice()),
        ("roads.dbf", attributes.as_slice()),
        ("roads.shx", b"index".as_slice()),
        ("roads.prj", br#"GEOGCS["WGS 84"]"#.as_slice()),
        ("roads.txt", b"notes".as_slice()),
    ] {
        fs::write(root.join(path), bytes).expect("write fixture member");
    }
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan");
    let descriptors = store
        .project_file_descriptors(scan.project.id)
        .expect("persisted file identities");
    assert!(
        descriptors
            .iter()
            .filter(|file| file.relative_path.starts_with("roads."))
            .take(4)
            .all(|file| file.grouping_key.as_deref() == Some("shapefile:roads"))
    );
    let source_entity = descriptors
        .into_iter()
        .find(|file| file.relative_path == "roads.shp")
        .expect("Shapefile primary source")
        .entity;
    let expected = scan
        .files
        .iter()
        .map(|file| {
            (
                file.relative_path.clone(),
                *blake3::hash(&file.bytes).as_bytes(),
            )
        })
        .collect::<Vec<_>>();
    let imported = service.import(scan.project.id).expect("import artifacts");
    for path in [
        "roads.shp",
        "roads.dbf",
        "roads.shx",
        "roads.prj",
        "roads.txt",
    ] {
        fs::remove_file(root.join(path)).expect("remove live source");
    }

    let artifacts = service
        .artifacts(scan.project.id)
        .expect("persisted artifacts");
    let roads = artifacts
        .iter()
        .find(|artifact| artifact.logical_key == "shapefile:roads")
        .expect("grouped Shapefile");
    assert_eq!(roads.kind, ArtifactKind::Shapefile);
    assert_eq!(roads.members.len(), 4);
    assert!(
        roads
            .members
            .iter()
            .all(|member| member.project_path.is_some())
    );
    let grouped_bytes = roads.canonical_bytes().expect("encode grouped artifact");
    let grouped_copy: scriberig_project::ArtifactManifest =
        serde_json::from_slice(&grouped_bytes).expect("decode grouped artifact");
    assert_eq!(grouped_copy, *roads);
    assert_eq!(
        grouped_copy.canonical_bytes().expect("re-encode"),
        grouped_bytes
    );
    assert!(artifacts.iter().any(|artifact| {
        artifact.logical_key == "file:roads.txt" && artifact.members.len() == 1
    }));
    for (path, digest) in expected {
        let member = artifacts
            .iter()
            .flat_map(|artifact| &artifact.members)
            .find(|member| member.key.0 == path)
            .expect("persisted member");
        assert_eq!(member.digest, digest);
    }
    assert_eq!(roads.id, ArtifactId::from_source_entity(source_entity));
    assert_ne!(roads.id.entity(), source_entity);
    assert!(
        store
            .resolve(imported.snapshot, roads.id.entity())
            .expect("resolve distinct artifact revision")
            .is_some()
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Downgrades an incomplete Shapefile group and reports missing required members.
#[test]
fn missing_shapefile_sidecars_produce_diagnostic_and_inspect_only_fidelity() {
    let root = temporary_project();
    fs::write(root.join("roads.shp"), shapefile_points()).expect("write primary member");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    service.import(scan.project.id).expect("import project");

    let artifact = service
        .artifacts(scan.project.id)
        .expect("artifact manifest")
        .into_iter()
        .find(|artifact| artifact.logical_key == "shapefile:roads")
        .expect("Shapefile group");
    assert_eq!(artifact.kind, ArtifactKind::Shapefile);
    assert_eq!(
        artifact.fidelity.capability,
        ArtifactCapability::InspectOnly
    );
    assert!(
        artifact
            .fidelity
            .diagnostic
            .as_deref()
            .is_some_and(|message| message.contains(".shx, .dbf"))
    );
    let diagnostics = service
        .artifact_grouping_diagnostics(scan.project.id)
        .expect("grouping diagnostics");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "shapefile-missing-required-sidecar"
            && diagnostic.logical_key == "shapefile:roads"
            && diagnostic.message.contains(".shx, .dbf")
    }));

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}

/// Encodes equivalent manifests deterministically and supports container parts.
#[test]
fn manifest_encoding_is_deterministic_and_supports_internal_members() {
    let root = temporary_project();
    fs::write(root.join("book.xlsx"), b"container").expect("write package");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan");
    let mut manifest = service
        .artifacts(scan.project.id)
        .expect("manifest")
        .remove(0);
    let filesystem_manifest = manifest.canonical_bytes().expect("encode file member");
    let filesystem_round_trip: scriberig_project::ArtifactManifest =
        serde_json::from_slice(&filesystem_manifest).expect("decode file member");
    assert_eq!(filesystem_round_trip, manifest);
    manifest.members = vec![scriberig_project::ArtifactMemberDescriptor {
        key: scriberig_project::ArtifactMemberKey("xl/workbook.xml".into()),
        project_path: None,
        container_part: Some("xl/workbook.xml".into()),
        digest: *blake3::hash(b"part").as_bytes(),
    }];
    let encoded = manifest.canonical_bytes().expect("encode");
    let decoded: scriberig_project::ArtifactManifest =
        serde_json::from_slice(&encoded).expect("decode");
    assert_eq!(decoded, manifest);
    assert_eq!(decoded.canonical_bytes().expect("re-encode"), encoded);
    assert!(decoded.members[0].project_path.is_none());

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
