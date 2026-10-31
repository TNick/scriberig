//! Tests stable project artifact identity through source renames.

use super::*;

/// Retains logical artifact identity across an unambiguous exact-source rename.
#[test]
fn artifact_identity_survives_unambiguous_source_rename() {
    let root = temporary_project();
    fs::write(root.join("original.bin"), b"stable exact source").expect("write source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let first_scan = service.scan(None, &root).expect("initial scan");
    let first_artifact = service
        .artifacts(first_scan.project.id)
        .expect("initial artifacts")
        .remove(0);
    let source_entity = store
        .project_file_descriptors(first_scan.project.id)
        .expect("initial source descriptor")
        .remove(0)
        .entity;
    service
        .import(first_scan.project.id)
        .expect("import original source");

    fs::rename(root.join("original.bin"), root.join("renamed.bin")).expect("rename source");
    service.scan(None, &root).expect("scan renamed source");
    let imported = service
        .import(first_scan.project.id)
        .expect("import renamed source");
    let renamed_artifact = service
        .artifacts(first_scan.project.id)
        .expect("renamed artifacts")
        .remove(0);
    let renamed_source = store
        .project_file_descriptors(first_scan.project.id)
        .expect("renamed source descriptor")
        .remove(0);

    assert_eq!(renamed_source.entity, source_entity);
    assert_eq!(renamed_artifact.id, first_artifact.id);
    assert!(
        store
            .resolve(imported.snapshot, renamed_artifact.id.entity())
            .expect("resolve artifact source handle")
            .is_some()
    );
    assert_eq!(renamed_artifact.logical_key, "file:renamed.bin");
    assert_eq!(
        renamed_artifact.members[0].project_path.as_deref(),
        Some("renamed.bin")
    );
    assert!(
        store
            .project_entities(first_scan.project.id)
            .expect("current semantic ownership")
            .iter()
            .any(|record| record.kind == "source"
                && record.relative_path == "renamed.bin"
                && record.entity == source_entity)
    );

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Preserves source and artifact identity when explicit rename evidence accompanies an edit.
#[test]
fn artifact_identity_survives_renamed_source_content_edit() {
    let root = temporary_project();
    fs::write(root.join("original.bin"), b"before edit").expect("write source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let first_scan = service.scan(None, &root).expect("initial scan");
    let first_artifact = service
        .artifacts(first_scan.project.id)
        .expect("initial artifacts")
        .remove(0);
    let source_entity = store
        .project_file_descriptors(first_scan.project.id)
        .expect("initial source descriptor")
        .remove(0)
        .entity;
    let original_import = service
        .import(first_scan.project.id)
        .expect("import original source");

    fs::rename(root.join("original.bin"), root.join("renamed.bin")).expect("rename source");
    fs::write(root.join("renamed.bin"), b"after edit").expect("edit renamed source");
    service
        .scan_with_renames(
            None,
            &root,
            ScanOptions::default(),
            &[ProjectFileRename {
                from_path: "original.bin".to_owned(),
                to_path: "renamed.bin".to_owned(),
            }],
        )
        .expect("scan explicit rename evidence");
    let imported = service
        .import(first_scan.project.id)
        .expect("import renamed edited source");
    let renamed_artifact = service
        .artifacts(first_scan.project.id)
        .expect("renamed artifacts")
        .remove(0);
    let renamed_source = store
        .project_file_descriptors(first_scan.project.id)
        .expect("renamed source descriptor")
        .remove(0);

    assert_eq!(renamed_source.entity, source_entity);
    assert_ne!(renamed_source.digest, first_artifact.members[0].digest);
    assert_eq!(renamed_artifact.id, first_artifact.id);
    assert_eq!(renamed_artifact.logical_key, "file:renamed.bin");
    let original_artifact_revision = store
        .resolve(original_import.snapshot, first_artifact.id.entity())
        .expect("resolve original artifact")
        .expect("original artifact revision");
    let edited_artifact_revision = store
        .resolve(imported.snapshot, renamed_artifact.id.entity())
        .expect("resolve edited artifact")
        .expect("edited artifact revision");
    assert_ne!(edited_artifact_revision, original_artifact_revision);

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove project");
}

/// Keeps case-colliding Shapefile candidates separate and reports the ambiguity.
#[test]
fn shapefile_case_collision_is_diagnostic_and_never_silently_grouped() {
    let root = temporary_project();
    for (path, bytes) in [
        ("roads.shp", b"primary lower".as_slice()),
        ("roads.dbf", b"attributes lower".as_slice()),
        ("Roads.SHP", b"primary upper".as_slice()),
        ("Roads.dbf", b"attributes upper".as_slice()),
    ] {
        fs::write(root.join(path), bytes).expect("write ambiguous member");
    }
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan candidates");
    let artifacts = service
        .artifacts(scan.project.id)
        .expect("artifact summaries");
    let diagnostics = service
        .artifact_grouping_diagnostics(scan.project.id)
        .expect("grouping diagnostics");

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "artifact-grouping-case-collision");
    assert_eq!(diagnostics[0].paths.len(), 4);
    assert!(diagnostics[0].message.contains("kept separate"));
    assert!(!artifacts.iter().any(|artifact| {
        artifact.kind == ArtifactKind::Shapefile && artifact.members.len() > 1
    }));
    assert_eq!(artifacts.len(), 4);

    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
