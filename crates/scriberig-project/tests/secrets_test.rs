//! Verifies sensitive sources are excluded before project persistence.

use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Scanning excludes credentials while retaining explicit environment templates.
#[test]
fn scan_and_import_exclude_sensitive_sources() {
    let temporary = tempfile::tempdir().expect("temporary project");
    let root = temporary.path().join("project");
    std::fs::create_dir(&root).expect("project directory");
    for name in [".env", ".env.local", "private.pem", ".npmrc"] {
        std::fs::write(root.join(name), "CANARY_MUST_NOT_PERSIST").expect("secret fixture");
    }
    std::fs::write(root.join(".env.example"), "EXAMPLE=placeholder").expect("template");
    std::fs::write(root.join("README.md"), "# Public\n").expect("public document");
    let store = Arc::new(SqliteStore::open(temporary.path().join("store.sqlite")).expect("store"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan");
    assert_eq!(scan.files.len(), 2);
    assert!(
        scan.files
            .iter()
            .all(|file| !file.bytes.windows(6).any(|value| value == b"CANARY"))
    );
    let report = service.import(scan.project.id).expect("import");
    assert_eq!(report.files, 2);
    let manifest = store.project_files(scan.project.id).expect("manifest");
    assert_eq!(manifest.len(), 2);
}
