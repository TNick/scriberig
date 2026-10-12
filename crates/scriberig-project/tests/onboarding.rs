//! Verifies durable multi-project scan and semantic import behavior.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Creates a unique temporary directory for one onboarding test.
///
/// # Parameters
/// - `label`: Test label.
///
/// # Returns
/// Created temporary directory path.
fn temporary_directory(label: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("scriberig-project-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("temporary project directory");
    path
}

/// Persists manifests, imports semantic children, and removes deleted ownership.
#[test]
fn scan_reopen_import_and_delete_are_durable() {
    let root = temporary_directory("lifecycle");
    fs::write(root.join("README.md"), "# Title\n\nParagraph.\n").expect("Markdown");
    fs::write(
        root.join("main.rs"),
        "fn main() { let value = 1; println!(\"{value}\"); }\n",
    )
    .expect("Rust");
    fs::write(root.join("notes.txt"), "unsupported source").expect("text");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));

    let scan = service
        .scan(Some("fixture".to_owned()), &root)
        .expect("scan");
    assert_eq!(scan.files.len(), 3);
    assert_eq!(
        store
            .project_files(scan.project.id)
            .expect("manifest")
            .len(),
        3
    );
    drop(service);
    let reopened = Arc::new(SqliteStore::open(&database).expect("reopen"));
    let service = ProjectService::new(Arc::clone(&reopened));
    let report = service.import(scan.project.id).expect("import");
    assert_eq!(report.files, 2);
    assert!(report.entities >= 4);
    assert_ne!(
        reopened
            .project_entities(scan.project.id)
            .expect("entities"),
        Vec::new()
    );

    fs::remove_file(root.join("main.rs")).expect("remove Rust source");
    service.scan(None, &root).expect("rescan");
    let second = service.import(scan.project.id).expect("second import");
    assert_ne!(report.snapshot, second.snapshot);
    assert!(
        reopened
            .project_entities(scan.project.id)
            .expect("updated entities")
            .iter()
            .all(|entity| !entity.relative_path.ends_with("main.rs"))
    );
}
