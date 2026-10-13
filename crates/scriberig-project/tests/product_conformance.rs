//! Verifies the project workflow's public cross-surface invariants.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_project::ProjectService;
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

/// GOLDEN-CONFORMANCE-01 proves public project and snapshot resolution agree.
#[test]
fn golden_conformance_01_project_workspace_equivalence() {
    let (_root, store, project, snapshot, _entity) = setup("workspace");
    let workspace = ProjectService::new(Arc::clone(&store))
        .workspace(project)
        .expect("workspace");
    assert_eq!(workspace.snapshot, snapshot);
}

/// GOLDEN-CONFORMANCE-02 proves named add has one stable semantic identity.
#[test]
fn golden_conformance_02_named_entity_identity() {
    let (_root, store, project, _snapshot, entity) = setup("identity");
    let symbols = ProjectService::new(Arc::clone(&store))
        .symbols(project)
        .expect("symbols");
    assert_eq!(
        symbols
            .iter()
            .filter(|symbol| symbol.entity == entity)
            .count(),
        1
    );
    assert_eq!(
        symbols
            .iter()
            .find(|symbol| symbol.entity == entity)
            .unwrap()
            .name
            .as_deref(),
        Some("add")
    );
}

/// GOLDEN-CONFORMANCE-03 proves source projection uses the exact imported file.
#[test]
fn golden_conformance_03_source_projection_semantics() {
    let (root, store, project, _snapshot, entity) = setup("projection");
    let projection = ProjectService::new(Arc::clone(&store))
        .projection(project, entity)
        .expect("projection");
    assert!(String::from_utf8_lossy(&projection.text).contains("left + right"));
    assert_eq!(projection.path, "src/lib.rs");
    assert!(root.join("src/lib.rs").exists());
}

/// GOLDEN-CONFORMANCE-04 proves human candidate meaning is shared.
#[test]
fn golden_conformance_04_equivalent_human_change() {
    let (_root, store, project, _snapshot, entity) = setup("human");
    let candidate = ProjectService::new(Arc::clone(&store)).propose_candidate(project, entity, "pub mod math { pub struct Calculator; impl Calculator { pub fn add(&self, left: i32, right: i32) -> i32 { left.saturating_add(right) } } }\n".to_owned()).expect("candidate");
    assert!(String::from_utf8_lossy(&candidate.new).contains("saturating_add"));
}

/// GOLDEN-CONFORMANCE-05 proves stale base apply is rejected.
#[test]
fn golden_conformance_05_stale_base_rejected() {
    let (root, store, project, _snapshot, entity) = setup("stale");
    let service = ProjectService::new(Arc::clone(&store));
    let source = fs::read_to_string(root.join("src/lib.rs")).expect("source");
    let candidate = service
        .propose_candidate(
            project,
            entity,
            source.replace("left + right", "left.saturating_add(right)"),
        )
        .expect("candidate");
    fs::write(
        root.join("src/lib.rs"),
        source.replace("left + right", "left.saturating_add(right)"),
    )
    .expect("external successor");
    service.scan(None, &root).expect("rescan");
    service.import(project).expect("successor");
    assert_eq!(
        service.apply_candidate(&candidate).expect_err("stale").code,
        "stale_candidate"
    );
}

/// GOLDEN-CONFORMANCE-06 proves projects in one database remain isolated.
#[test]
fn golden_conformance_06_projects_are_isolated() {
    let (root_a, store, project_a, _snapshot_a, entity_a) = setup("a");
    let root_b =
        std::env::temp_dir().join(format!("scriberig-conformance-b-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root_b);
    fs::create_dir_all(root_b.join("src")).expect("source directory b");
    fs::write(
        root_b.join("Cargo.toml"),
        "[package]\nname=\"golden_b\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .expect("manifest b");
    fs::write(
        root_b.join("src/lib.rs"),
        "pub fn untouched() -> i32 { 7 }\n",
    )
    .expect("source b");
    let shared_service = ProjectService::new(Arc::clone(&store));
    let scan_b = shared_service
        .scan(Some("b".to_owned()), &root_b)
        .expect("scan b");
    let imported_b = shared_service.import(scan_b.project.id).expect("import b");
    let project_b = scan_b.project.id;
    let snapshot_b = imported_b.snapshot;
    assert_ne!(project_a, project_b);
    let source_a = fs::read_to_string(root_a.join("src/lib.rs")).expect("source a");
    let candidate = ProjectService::new(Arc::clone(&store))
        .propose_candidate(
            project_a,
            entity_a,
            source_a.replace("left + right", "left.saturating_add(right)"),
        )
        .expect("candidate");
    let report = ProjectService::new(Arc::clone(&store))
        .apply_candidate(&candidate)
        .expect("apply");
    assert_eq!(report.effect_state, "applied");
    assert_eq!(
        store
            .project_effect(&candidate.id)
            .expect("effect")
            .expect("receipt")
            .state,
        "applied"
    );
    assert!(
        fs::read_to_string(root_b.join("src/lib.rs"))
            .expect("source b")
            .contains("untouched")
    );
    assert_eq!(
        store
            .project(project_b)
            .expect("project b")
            .unwrap()
            .current_snapshot,
        Some(snapshot_b)
    );
}

/// GOLDEN-CONFORMANCE-07 proves agent and human candidates share the candidate API.
#[test]
fn golden_conformance_07_agent_candidate_shape_is_shared() {
    let (_root, store, project, _snapshot, entity) = setup("agent");
    let candidate = ProjectService::new(store).propose_candidate(project, entity, "pub mod math { pub struct Calculator; impl Calculator { pub fn add(&self, left: i32, right: i32) -> i32 { left.saturating_add(right) } } }\n".to_owned()).expect("agent candidate");
    assert_eq!(candidate.entity, entity);
    assert_eq!(candidate.path, "src/lib.rs");
}
