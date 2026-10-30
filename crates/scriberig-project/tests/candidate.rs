//! Verifies snapshot-bound candidate creation and filesystem conflict safety.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Creates a temporary Rust project for candidate tests.
///
/// The newly created temporary project directory.
/// # Returns
/// The operation result.
fn temporary_directory() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "scriberig-project-candidate-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(path.join("src")).expect("source directory");
    path
}

/// Verifies candidates remain snapshot-bound and protect external edits.
#[test]
fn source_candidate_is_snapshot_bound_and_conflict_safe() {
    let root = temporary_directory();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"candidate\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .expect("manifest");
    let base = "pub fn add(left: i32, right: i32) -> i32 { left + right }\n";
    fs::write(root.join("src/lib.rs"), base).expect("Rust source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service
        .scan(Some("candidate".to_owned()), &root)
        .expect("scan");
    let imported = service.import(scan.project.id).expect("import");
    let entity = store
        .project_entities(scan.project.id)
        .expect("entities")
        .into_iter()
        .find(|record| record.name.as_deref() == Some("add"))
        .expect("named add")
        .entity;
    let edited = base.replace("left + right", "left.saturating_add(right)");
    let candidate = service
        .propose_candidate(scan.project.id, entity, edited.clone())
        .expect("candidate");
    let sibling_candidate = service
        .propose_candidate(
            scan.project.id,
            entity,
            base.replace("left + right", "left + (right * 1)"),
        )
        .expect("sibling candidate");
    let reopened = Arc::new(SqliteStore::open(&database).expect("reopen database"));
    let reopened_candidate = ProjectService::new(Arc::clone(&reopened))
        .candidate(&candidate.id)
        .expect("load durable candidate")
        .expect("durable candidate");
    assert_eq!(reopened_candidate, candidate);
    assert_eq!(candidate.base_snapshot, imported.snapshot);
    assert_eq!(sibling_candidate.base_snapshot, imported.snapshot);
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).expect("source"),
        base
    );
    assert_eq!(
        store
            .project(scan.project.id)
            .expect("project")
            .expect("record")
            .current_snapshot,
        Some(imported.snapshot)
    );
    fs::write(root.join("src/lib.rs"), format!("// external edit\n{base}")).expect("external edit");
    let error = service.apply_candidate(&candidate).expect_err("conflict");
    assert_eq!(error.code, "filesystem_conflict");
    let effect = reopened
        .project_effect(&candidate.id)
        .expect("effect receipt")
        .expect("durable effect receipt");
    assert_eq!(effect.state, "failed");
    assert_eq!(effect.base_snapshot, candidate.base_snapshot);
    assert!(
        fs::read_to_string(root.join("src/lib.rs"))
            .expect("source")
            .contains("external edit")
    );
    fs::write(root.join("src/lib.rs"), base).expect("restore exact candidate base");
    assert!(candidate.validation_digest.is_some());
    let applied = service
        .apply_candidate(&candidate)
        .expect("apply directly validated source candidate");
    assert_ne!(applied.successor_snapshot, imported.snapshot);
    assert_eq!(
        fs::read_to_string(root.join("src/lib.rs")).expect("materialized source"),
        edited
    );
}
