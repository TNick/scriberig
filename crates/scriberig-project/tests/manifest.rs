//! Verifies exact digest-bound project manifest review and import.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Creates a unique temporary project directory.
///
/// # Returns
/// The created temporary directory.
fn temporary_project() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "scriberig-manifest-{}",
        scriberig_core::EntityId::new()
    ));
    fs::create_dir_all(&root).expect("create fixture");
    root
}

/// Reviews bounded manifest entries and rejects import after its exact scan changes.
#[test]
fn manifest_pages_bind_import_to_reviewed_scan() {
    let root = temporary_project();
    fs::write(root.join("Cargo.toml"), "[package]\nname='fixture'\n").expect("manifest");
    fs::write(root.join("src.rs"), "fn main() {}\n").expect("source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan");
    let summary = service
        .manifest_summary(scan.project.id)
        .expect("manifest summary");
    assert_eq!(summary.file_count, 2);
    assert_eq!(summary.ready_count, 2);
    let page = service
        .manifest_page(scan.project.id, &summary.manifest_id, 0, 1)
        .expect("manifest page");
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.next_offset, Some(1));
    assert!(page.entries.iter().all(|entry| !entry.path.is_empty()));
    assert!(
        service
            .import_manifest(scan.project.id, &summary.manifest_id)
            .is_ok()
    );
    let snapshot = service
        .workspace(scan.project.id)
        .expect("workspace")
        .snapshot;
    let source = service
        .symbols_at(scan.project.id, snapshot)
        .expect("snapshot symbols")
        .into_iter()
        .find(|symbol| symbol.path == "src.rs" && symbol.kind == "source")
        .expect("source symbol");
    fs::write(root.join("src.rs"), "fn main() { let changed = true; }\n").expect("change source");
    let changed = service.scan(None, &root).expect("rescan");
    assert_eq!(
        service
            .projection_at(scan.project.id, snapshot, source.entity)
            .expect("exact snapshot source")
            .text,
        b"fn main() {}\n"
    );
    let changed_summary = service
        .manifest_summary(changed.project.id)
        .expect("updated summary");
    assert_ne!(changed_summary.manifest_id, summary.manifest_id);
    assert_eq!(
        service
            .import_manifest(scan.project.id, &summary.manifest_id)
            .expect_err("stale manifest rejected")
            .code,
        "manifest_changed"
    );
    assert_eq!(
        store
            .project(scan.project.id)
            .expect("project after rejected import")
            .expect("persisted project")
            .current_snapshot,
        Some(
            service
                .workspace(scan.project.id)
                .expect("authoritative workspace")
                .snapshot,
        )
    );
    let successor = service
        .import_manifest(scan.project.id, &changed_summary.manifest_id)
        .expect("import successor snapshot");
    assert_ne!(successor.snapshot, snapshot);
    assert_eq!(
        service
            .projection_at(scan.project.id, snapshot, source.entity)
            .expect_err("old source reference must not retarget")
            .code,
        "stale_snapshot"
    );
    drop(service);
    drop(store);
    fs::remove_file(database).expect("remove database");
    fs::remove_dir_all(root).expect("remove fixture");
}
