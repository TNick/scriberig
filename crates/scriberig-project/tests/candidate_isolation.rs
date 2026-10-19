//! Verifies aggregate candidate isolation across project roots and external edits.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_project::{ProjectService, ProjectSourceChange, ProjectSourceChangeKind};
use scriberig_store_sqlite::SqliteStore;

/// Creates one imported temporary Rust project.
///
/// - `label`: Unique fixture label.
///
/// Project root, shared store, project identity, and immutable base snapshot.
/// # Returns
/// The operation result.
fn setup(
    label: &str,
) -> (
    PathBuf,
    Arc<SqliteStore>,
    scriberig_core::ProjectId,
    scriberig_core::SnapshotId,
) {
    let root = std::env::temp_dir().join(format!(
        "scriberig-candidate-isolation-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"candidate_isolation\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), "pub fn base() -> u8 { 1 }\n").expect("source");
    let store =
        Arc::new(SqliteStore::open(root.with_extension("scriberig.sqlite")).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(Some(label.to_owned()), &root).expect("scan");
    let imported = service.import(scan.project.id).expect("import");
    (root, store, scan.project.id, imported.snapshot)
}

/// MULTIPROJECT-PROJECT-APPLY-01 scopes aggregate effects to one project root.
#[test]
fn aggregate_apply_leaves_other_project_and_snapshot_unchanged() {
    let (root_a, store, project_a, snapshot_a) = setup("multi-apply-a");
    let root_b = std::env::temp_dir().join(format!(
        "scriberig-candidate-isolation-multi-b-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root_b);
    fs::create_dir_all(root_b.join("src")).expect("other project source directory");
    fs::write(
        root_b.join("Cargo.toml"),
        "[package]\nname=\"other_project\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .expect("other project manifest");
    let other_bytes = b"pub fn untouched() -> u8 { 9 }\n";
    fs::write(root_b.join("src/lib.rs"), other_bytes).expect("other project source");
    let service = ProjectService::new(Arc::clone(&store));
    let scan_b = service
        .scan(Some("multi-apply-b".to_owned()), &root_b)
        .expect("scan other project");
    let snapshot_b = service
        .import(scan_b.project.id)
        .expect("import other project")
        .snapshot;
    let candidate = service
        .propose_project_change(
            project_a,
            snapshot_a,
            vec![
                ProjectSourceChange {
                    path: "src/lib.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Update,
                    content: Some(b"pub fn base() -> u8 { 2 }\n".to_vec()),
                },
                ProjectSourceChange {
                    path: "src/generated.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Create,
                    content: Some(b"pub fn generated() {}\n".to_vec()),
                },
            ],
        )
        .expect("project A candidate");
    let candidate = service
        .validate_candidate(&candidate.id)
        .expect("candidate validation");
    service
        .apply_candidate(&candidate)
        .expect("apply to project A");
    assert_eq!(
        fs::read(root_b.join("src/lib.rs")).expect("other source remains"),
        other_bytes
    );
    assert!(!root_b.join("src/generated.rs").exists());
    assert_eq!(
        store
            .project(scan_b.project.id)
            .expect("read other project")
            .expect("other project exists")
            .current_snapshot,
        Some(snapshot_b)
    );
    assert_eq!(
        fs::read(root_a.join("src/lib.rs")).expect("project A update"),
        b"pub fn base() -> u8 { 2 }\n"
    );
    assert!(root_a.join("src/generated.rs").exists());
}

/// Verifies that an external edit blocks a candidate deletion and earlier update.
#[test]
fn external_edit_to_delete_target_is_preserved() {
    let (root, store, project, base_snapshot) = setup("delete-conflict");
    fs::write(
        root.join("src/obsolete.rs"),
        "pub fn obsolete() -> u8 { 1 }\n",
    )
    .expect("obsolete source");
    let service = ProjectService::new(Arc::clone(&store));
    service
        .scan(None, &root)
        .expect("rescan with obsolete source");
    let snapshot = service
        .import(project)
        .expect("import obsolete source")
        .snapshot;
    assert_ne!(base_snapshot, snapshot);
    let candidate = service
        .propose_project_change(
            project,
            snapshot,
            vec![
                ProjectSourceChange {
                    path: "src/lib.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Update,
                    content: Some(b"pub fn base() -> u8 { 2 }\n".to_vec()),
                },
                ProjectSourceChange {
                    path: "src/obsolete.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Delete,
                    content: None,
                },
            ],
        )
        .expect("candidate deleting obsolete source");
    let candidate = service
        .validate_candidate(&candidate.id)
        .expect("candidate validation");
    let external = b"external content must survive\n";
    fs::write(root.join("src/obsolete.rs"), external).expect("external update");
    assert_eq!(
        service
            .apply_candidate(&candidate)
            .expect_err("delete target changed")
            .code,
        "filesystem_conflict"
    );
    assert_eq!(
        fs::read(root.join("src/obsolete.rs")).expect("external deletion target remains"),
        external
    );
    assert_eq!(
        fs::read(root.join("src/lib.rs")).expect("unrelated candidate update untouched"),
        b"pub fn base() -> u8 { 1 }\n"
    );
}
