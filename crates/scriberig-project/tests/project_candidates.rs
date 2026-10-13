//! Verifies exact-base aggregate project candidate behavior.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_project::{
    ProjectService, ProjectSourceChange, ProjectSourceChangeKind, ProjectSourceEffectKind,
};
use scriberig_store_sqlite::SqliteStore;

/// Creates a temporary imported project and returns its public identities.
///
/// # Parameters
/// - `label`: Unique fixture label.
///
/// # Returns
/// Fixture root, store, project, snapshot, and named `add` entity.
fn setup(
    label: &str,
) -> (
    PathBuf,
    Arc<SqliteStore>,
    scriberig_core::ProjectId,
    scriberig_core::SnapshotId,
    scriberig_core::EntityId,
) {
    let root = std::env::temp_dir().join(format!(
        "scriberig-conformance-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname=\"golden\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .expect("manifest");
    fs::write(
        root.join("src/lib.rs"),
        "pub mod math { pub struct Calculator; impl Calculator { pub fn add(&self, left: i32, right: i32) -> i32 { left + right } } }\n",
    )
    .expect("source");
    let database = root.join("scriberig.sqlite");
    let store = Arc::new(SqliteStore::open(database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(Some(label.to_owned()), &root).expect("scan");
    let imported = service.import(scan.project.id).expect("import");
    let entity = service
        .symbols(scan.project.id)
        .expect("symbols")
        .into_iter()
        .find(|symbol| symbol.name.as_deref() == Some("add"))
        .expect("named add")
        .entity;
    (root, store, scan.project.id, imported.snapshot, entity)
}

/// PROJECT-CANDIDATE-01 and PROJECT-DURABLE-01 prove aggregate creation is durable and inert.
#[test]
fn project_candidate_persists_create_and_update_without_touching_live_tree() {
    let (root, store, project, snapshot, _entity) = setup("aggregate");
    let manifest = fs::read(root.join("Cargo.toml")).expect("manifest");
    let service = ProjectService::new(Arc::clone(&store));
    let candidate = service
        .propose_project_change(
            project,
            snapshot,
            vec![
                ProjectSourceChange {
                    path: "src/new_module.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Create,
                    content: Some(b"pub fn added() {}\n".to_vec()),
                },
                ProjectSourceChange {
                    path: "Cargo.toml".to_owned(),
                    kind: ProjectSourceChangeKind::Update,
                    content: Some(
                        b"[package]\nname=\"golden\"\nversion=\"0.1.0\"\nedition=\"2021\"\n"
                            .to_vec(),
                    ),
                },
            ],
        )
        .expect("aggregate candidate");
    assert_eq!(candidate.base_snapshot, snapshot);
    assert_eq!(candidate.effects.len(), 2);
    assert_eq!(candidate.effects[0].kind, ProjectSourceEffectKind::Update);
    assert_eq!(candidate.effects[1].kind, ProjectSourceEffectKind::Create);
    assert!(!root.join("src/new_module.rs").exists());
    assert_eq!(
        fs::read(root.join("Cargo.toml")).expect("unchanged manifest"),
        manifest
    );
    assert_eq!(
        ProjectService::new(Arc::clone(&store))
            .workspace(project)
            .expect("unchanged workspace")
            .snapshot,
        snapshot
    );
    drop(service);
    drop(store);
    let reopened_store =
        Arc::new(SqliteStore::open(root.join("scriberig.sqlite")).expect("reopen database"));
    let reopened = ProjectService::new(Arc::clone(&reopened_store))
        .candidate(&candidate.id)
        .expect("read candidate")
        .expect("durable candidate");
    assert_eq!(reopened.effects, candidate.effects);
    assert_eq!(reopened.base_snapshot, snapshot);
    assert_eq!(
        reopened.effects[1].new.as_deref(),
        Some(b"pub fn added() {}\n".as_slice())
    );
}

/// PROJECT-CANDIDATE negative cases reject ambiguous and escaping paths.
#[test]
fn project_candidate_rejects_duplicate_and_escaping_paths() {
    let (_root, store, project, snapshot, _entity) = setup("invalid-aggregate");
    let service = ProjectService::new(store);
    let duplicate = vec![
        ProjectSourceChange {
            path: "src/new.rs".to_owned(),
            kind: ProjectSourceChangeKind::Create,
            content: Some(Vec::new()),
        },
        ProjectSourceChange {
            path: "src/new.rs".to_owned(),
            kind: ProjectSourceChangeKind::Create,
            content: Some(Vec::new()),
        },
    ];
    assert_eq!(
        service
            .propose_project_change(project, snapshot, duplicate)
            .expect_err("duplicate path")
            .code,
        "duplicate_effect_path"
    );
    assert_eq!(
        service
            .propose_project_change(
                project,
                snapshot,
                vec![ProjectSourceChange {
                    path: "../outside".to_owned(),
                    kind: ProjectSourceChangeKind::Create,
                    content: Some(Vec::new()),
                }],
            )
            .expect_err("escaping path")
            .code,
        "unsafe_project_path"
    );
}

/// PROJECT-CANDIDATE exact-base sources ignore a later unimported filesystem scan.
#[test]
fn project_candidate_uses_immutable_base_bytes_after_rescan() {
    let (root, store, project, snapshot, _entity) = setup("exact-base-effects");
    let base = fs::read(root.join("src/lib.rs")).expect("base source");
    fs::write(root.join("src/lib.rs"), b"unimported external bytes\n").expect("external edit");
    let service = ProjectService::new(store);
    service.scan(None, &root).expect("rescan without import");
    let candidate = service
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/lib.rs".to_owned(),
                kind: ProjectSourceChangeKind::Update,
                content: Some(b"candidate bytes\n".to_vec()),
            }],
        )
        .expect("exact-base candidate");
    assert_eq!(candidate.effects[0].old.as_deref(), Some(base.as_slice()));
    assert_eq!(candidate.base_snapshot, snapshot);
}

/// PROJECT-CANDIDATE-03 proves deletion records exact base bytes without removing the file.
#[test]
fn project_candidate_delete_remains_a_reviewable_effect() {
    let (root, store, project, snapshot, _entity) = setup("delete-aggregate");
    let source = fs::read(root.join("src/lib.rs")).expect("source");
    let candidate = ProjectService::new(Arc::clone(&store))
        .propose_project_change(
            project,
            snapshot,
            vec![ProjectSourceChange {
                path: "src/lib.rs".to_owned(),
                kind: ProjectSourceChangeKind::Delete,
                content: None,
            }],
        )
        .expect("delete candidate");
    assert_eq!(candidate.effects[0].kind, ProjectSourceEffectKind::Delete);
    assert_eq!(candidate.effects[0].old.as_deref(), Some(source.as_slice()));
    assert_eq!(candidate.effects[0].new, None);
    assert_eq!(
        ProjectService::new(Arc::clone(&store))
            .apply_candidate(&candidate)
            .expect_err("aggregate delete must not use single-file apply")
            .code,
        "aggregate_apply_unavailable"
    );
    assert_eq!(
        fs::read(root.join("src/lib.rs")).expect("live source"),
        source
    );
}

/// PROJECT-WORKSPACE-VALIDATE-01/02 validate the exact overlay without changing live files.
#[test]
fn project_validation_compiles_candidate_overlay_in_isolation() {
    let (root, store, project, snapshot, _entity) = setup("candidate-validation");
    let live_source = fs::read(root.join("src/lib.rs")).expect("base source");
    let candidate = ProjectService::new(Arc::clone(&store))
        .propose_project_change(
            project,
            snapshot,
            vec![
                ProjectSourceChange {
                    path: "src/lib.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Update,
                    content: Some(
                        format!("mod generated;\n{}", String::from_utf8_lossy(&live_source))
                            .into_bytes(),
                    ),
                },
                ProjectSourceChange {
                    path: "src/generated.rs".to_owned(),
                    kind: ProjectSourceChangeKind::Create,
                    content: Some(b"pub fn generated() -> u8 { 1 }\n".to_vec()),
                },
            ],
        )
        .expect("candidate");
    let validated = ProjectService::new(Arc::clone(&store))
        .validate_candidate(&candidate.id)
        .expect("validate candidate");
    assert_eq!(
        validated.validation,
        scriberig_project::CandidateValidation::Passed
    );
    assert_eq!(
        fs::read(root.join("src/lib.rs")).expect("live source"),
        live_source
    );
    assert!(!root.join("src/generated.rs").exists());
    assert_eq!(
        ProjectService::new(store)
            .workspace(project)
            .expect("base workspace")
            .snapshot,
        snapshot
    );
}
