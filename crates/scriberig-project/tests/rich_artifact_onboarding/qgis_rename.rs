//! Verifies candidate-only QGIS layer renames preserve exact project scope.

use std::fs;
use std::io::{Read, Write};
use std::sync::Arc;

use scriberig_project::{CandidateValidation, CandidateValidationProfile, ProjectService};
use scriberig_store_sqlite::SqliteStore;
use zip::write::SimpleFileOptions;

use super::support::mixed_project;

/// Renames one QGIS display reference through validation and shared apply.
#[test]
fn qgis_layer_rename_is_candidate_only_and_preserves_other_references() {
    let root = mixed_project();
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let imported = service.import(project).expect("import project");
    let qgis = service
        .qgis_project(project, "project.qgs", imported.snapshot)
        .expect("exact QGIS project");
    let target = qgis
        .project
        .layers
        .iter()
        .find(|layer| layer.name == "Parcels")
        .expect("parcel reference");
    let target_layer = target.layer;
    let target_dataset = target.dataset;
    let original_bytes = fs::read(root.join("project.qgs")).expect("source project bytes");
    let candidate = service
        .propose_qgis_layer_rename(project, imported.snapshot, target.id, "Lots & Parcels")
        .expect("propose layer rename");
    assert_eq!(
        candidate.semantic_operation.as_deref(),
        Some("qgis.rename_layer_reference")
    );
    super::semantic_candidate_assertions::assert_persisted_semantic_change(
        &service,
        &candidate,
        target.id,
        "layer_reference.name",
        &serde_json::json!("Parcels"),
        &serde_json::json!("Lots & Parcels"),
    );
    assert_eq!(candidate.validation, CandidateValidation::Pending);
    assert_eq!(
        fs::read(root.join("project.qgs")).expect("live source before apply"),
        original_bytes
    );

    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::QgisPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate QGIS rename");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = service
        .apply_candidate(&validated)
        .expect("apply validated rename");
    let reopened = service
        .qgis_project(project, "project.qgs", applied.successor_snapshot)
        .expect("read successor QGIS project");
    let renamed = reopened
        .project
        .layers
        .iter()
        .find(|layer| layer.id == target.id)
        .expect("renamed layer reference");
    assert_eq!(renamed.name, "Lots & Parcels");
    assert_eq!(renamed.layer, target_layer);
    assert_eq!(renamed.dataset, target_dataset);
    assert!(
        reopened
            .project
            .layers
            .iter()
            .any(|layer| { layer.id != target.id && layer.name == "Roads" })
    );
    assert_ne!(applied.successor_snapshot, imported.snapshot);

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove temporary database");
    fs::remove_dir_all(root).expect("remove temporary project");
}

/// Applies a QGZ container edit through shared artifact validation/materialization.
#[test]
fn qgz_rename_uses_shared_container_evidence_and_preserves_parts() {
    let root = mixed_project();
    let qgs_path = root.join("project.qgs");
    let project_xml = fs::read(&qgs_path).expect("source QGS bytes");
    fs::remove_file(&qgs_path).expect("remove standalone project");
    let qgz_path = root.join("project.qgz");
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file("project.qgs", SimpleFileOptions::default())
        .expect("QGS package member");
    writer.write_all(&project_xml).expect("write QGS member");
    writer
        .start_file("styles/custom.qml", SimpleFileOptions::default())
        .expect("custom style package member");
    writer
        .write_all(b"opaque style payload")
        .expect("write custom style");
    let original = writer.finish().expect("finish QGZ").into_inner();
    fs::write(&qgz_path, &original).expect("write QGZ source");

    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan project");
    let project = scan.project.id;
    let imported = service.import(project).expect("import project");
    let qgis = service
        .qgis_project(project, "project.qgz", imported.snapshot)
        .expect("exact QGZ project");
    let target = qgis
        .project
        .layers
        .iter()
        .find(|layer| layer.name == "Parcels")
        .expect("parcel layer reference");
    let candidate = service
        .propose_qgis_layer_rename(project, imported.snapshot, target.id, "Lots")
        .expect("QGZ rename candidate");
    let validated = service
        .validate_candidate_profile_cancellable(
            &candidate.id,
            CandidateValidationProfile::QgisPackage,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .expect("validate QGZ rename");
    assert_eq!(validated.validation, CandidateValidation::Passed);
    let applied = service
        .apply_candidate(&validated)
        .expect("materialize QGZ candidate");

    let output = fs::read(&qgz_path).expect("materialized QGZ");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&output)).expect("open output");
    let mut style_bytes = Vec::new();
    archive
        .by_name("styles/custom.qml")
        .expect("preserved style part")
        .read_to_end(&mut style_bytes)
        .expect("read preserved style");
    assert_eq!(style_bytes, b"opaque style payload");
    let reopened = service
        .qgis_project(project, "project.qgz", applied.successor_snapshot)
        .expect("reopen materialized QGZ");
    assert_eq!(
        reopened
            .project
            .layers
            .iter()
            .find(|layer| layer.id == target.id)
            .expect("renamed reference")
            .name,
        "Lots"
    );
    let manifest = service
        .artifacts(project)
        .expect("updated artifact manifest")
        .into_iter()
        .find(|manifest| manifest.logical_key == "file:project.qgz")
        .expect("QGZ artifact");
    let style = manifest
        .members
        .iter()
        .find(|member| member.container_part.as_deref() == Some("styles/custom.qml"))
        .expect("preserved part evidence");
    assert_eq!(style.digest, *blake3::hash(&style_bytes).as_bytes());

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove project store");
    fs::remove_dir_all(root).expect("remove project fixture");
}
