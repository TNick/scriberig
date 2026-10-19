//! Verifies aggregate candidate validation and materialization safety.

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
        "scriberig-candidate-safety-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"candidate_safety\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
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

/// Creates and validates one candidate update to the base Rust source.
///
/// - `service`: Shared project workflow.
/// - `project`: Project receiving the candidate.
/// - `snapshot`: Exact base snapshot.
///
/// Candidate after isolated validation succeeds.
/// # Returns
/// The operation result.
fn validated_update(
    service: &ProjectService,
    project: scriberig_core::ProjectId,
    snapshot: scriberig_core::SnapshotId,
) -> scriberig_project::ProjectCandidate {
    let candidate = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/lib.rs".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"pub fn base() -> u8 { 2 }\n".to_vec()),
            }],
        )
        .expect("candidate");
    service
        .validate_candidate(&candidate.id)
        .expect("validation")
}

/// PROJECT-CONFLICT-UPDATE-01 preserves an external update to a candidate path.
#[test]
fn external_update_conflict_preserves_bytes_and_records_path_state() {
    let (root, store, project, snapshot) = setup("update-conflict");
    let service = ProjectService::new(Arc::clone(&store));
    let candidate = validated_update(&service, project, snapshot);
    let external = b"external edit remains\n";
    fs::write(root.join("src/lib.rs"), external).expect("external edit");
    let error = service.apply_candidate(&candidate).expect_err("conflict");
    assert_eq!(error.code, "filesystem_conflict");
    assert_eq!(
        fs::read(root.join("src/lib.rs")).expect("preserved edit"),
        external
    );
    let receipt = store
        .project_effect_paths(&candidate.id)
        .expect("path receipt");
    assert_eq!(receipt[0].state, "conflict");
    assert_eq!(
        store
            .project_effect(&candidate.id)
            .expect("effect")
            .expect("receipt")
            .state,
        "failed"
    );
}

/// PROJECT-CONFLICT-CREATE-01 preserves an externally created candidate path.
#[test]
fn external_create_conflict_is_not_overwritten() {
    let (root, store, project, snapshot) = setup("create-conflict");
    let service = ProjectService::new(Arc::clone(&store));
    let candidate = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/new.rs".to_owned(),
                kind: ProjectSourceChangeKind::Create,
                content: Some(b"pub fn candidate() {}\n".to_vec()),
            }],
        )
        .expect("candidate");
    let candidate = service
        .validate_candidate(&candidate.id)
        .expect("validation");
    let external = b"externally created\n";
    fs::write(root.join("src/new.rs"), external).expect("external creation");
    assert_eq!(
        service
            .apply_candidate(&candidate)
            .expect_err("conflict")
            .code,
        "filesystem_conflict"
    );
    assert_eq!(
        fs::read(root.join("src/new.rs")).expect("preserved file"),
        external
    );
}

/// Aggregate preflight checks every path before materializing the first effect.
#[test]
fn aggregate_conflict_prevents_unrelated_candidate_creation() {
    let (root, store, project, snapshot) = setup("aggregate-preflight");
    let service = ProjectService::new(Arc::clone(&store));
    let candidate = service
        .propose_project_change(
            project,
            snapshot,
            vec![
                ProjectSourceChange {
                    path: "src/generated.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Create,
                    content: Some(b"pub fn generated() {}\n".to_vec()),
                },
                ProjectSourceChange {
                    path: "src/lib.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Update,
                    content: Some(b"pub fn base() -> u8 { 2 }\n".to_vec()),
                },
            ],
        )
        .expect("candidate");
    let candidate = service
        .validate_candidate(&candidate.id)
        .expect("validation");
    let external = b"pub fn external() {}\n";
    fs::write(root.join("src/lib.rs"), external).expect("external edit");
    assert_eq!(
        service
            .apply_candidate(&candidate)
            .expect_err("preflight conflict")
            .code,
        "filesystem_conflict"
    );
    assert!(!root.join("src/generated.rs").exists());
    assert_eq!(
        fs::read(root.join("src/lib.rs")).expect("preserved external source"),
        external
    );
}

/// PROJECT-APPLY-STALE-01 rejects an aggregate candidate after canonical advance.
#[test]
fn stale_aggregate_candidate_does_not_materialize_effects() {
    let (root, store, project, snapshot) = setup("stale-aggregate");
    let service = ProjectService::new(Arc::clone(&store));
    let candidate = validated_update(&service, project, snapshot);
    let external = b"pub fn external() {}\n";
    fs::write(root.join("src/lib.rs"), external).expect("successor bytes");
    service.scan(None, &root).expect("rescan");
    service.import(project).expect("successor import");
    assert_eq!(
        service
            .apply_candidate(&candidate)
            .expect_err("stale candidate")
            .code,
        "stale_candidate"
    );
    assert_eq!(
        fs::read(root.join("src/lib.rs")).expect("successor bytes preserved"),
        external
    );
    assert!(!root.join("src/new.rs").exists());
}

/// PROJECT-WORKSPACE-VALIDATE-03 refuses to apply a failed candidate.
#[test]
fn validation_failure_keeps_candidate_inapplicable() {
    let (root, store, project, snapshot) = setup("validation-failure");
    let service = ProjectService::new(Arc::clone(&store));
    let candidate = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/lib.rs".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"pub fn base( {\n".to_vec()),
            }],
        )
        .expect("candidate");
    let candidate = service
        .validate_candidate(&candidate.id)
        .expect("validation result");
    assert!(matches!(
        candidate.validation,
        scriberig_project::CandidateValidation::Failed { .. }
    ));
    assert_eq!(
        service
            .apply_candidate(&candidate)
            .expect_err("failed validation")
            .code,
        "validation_failed"
    );
    assert_eq!(
        fs::read(root.join("src/lib.rs")).expect("unchanged live source"),
        b"pub fn base() -> u8 { 1 }\n"
    );
}

/// PROJECT-WORKSPACE-VALIDATE-03 binds validation to the exact aggregate content identity.
#[test]
fn changed_candidate_cannot_reuse_prior_validation() {
    let (_root, store, project, snapshot) = setup("changed-validation");
    let service = ProjectService::new(Arc::clone(&store));
    let validated = validated_update(&service, project, snapshot);
    let changed = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/lib.rs".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"pub fn base() -> u8 { 3 }\n".to_vec()),
            }],
        )
        .expect("changed candidate");
    assert_ne!(validated.id, changed.id);
    assert_eq!(
        changed.validation,
        scriberig_project::CandidateValidation::Pending
    );
    assert_eq!(
        service
            .apply_candidate(&changed)
            .expect_err("pending validation")
            .code,
        "validation_pending"
    );
}

/// Rejects a parent-directory symlink that would redirect a candidate outside its project.
#[cfg(unix)]
#[test]
fn symlink_escape_is_rejected_before_materialization() {
    use std::os::unix::fs::symlink;

    let (root, store, project, snapshot) = setup("symlink-escape");
    let service = ProjectService::new(Arc::clone(&store));
    let candidate = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/escape/new.rs".to_owned(),
                kind: ProjectSourceChangeKind::Create,
                content: Some(b"pub fn escaped() {}\n".to_vec()),
            }],
        )
        .expect("candidate");
    let candidate = service
        .validate_candidate(&candidate.id)
        .expect("validation");
    let outside = root.with_extension("outside");
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&outside).expect("outside directory");
    symlink(&outside, root.join("src/escape")).expect("external symlink");
    assert_eq!(
        service
            .apply_candidate(&candidate)
            .expect_err("symlink escape")
            .code,
        "unsafe_project_path"
    );
    assert!(!outside.join("new.rs").exists());
}
