//! Verifies durable multi-project scan and semantic import behavior.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use scriberig_project::{ProjectService, ScanOptions};
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

/// Runs one Git command in a temporary repository.
///
/// # Parameters
/// - `root`: Repository working tree.
/// - `arguments`: Git arguments.
///
/// # Returns
/// Unit after Git succeeds.
fn git(root: &PathBuf, arguments: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .status()
        .expect("git executable");
    assert!(status.success(), "git command failed: {arguments:?}");
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
    fs::write(
        root.join("guide.html"),
        "<html><head><title>Guide</title></head><body><h1>Title</h1><p>Text</p></body></html>",
    )
    .expect("HTML");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));

    let scan = service
        .scan(Some("fixture".to_owned()), &root)
        .expect("scan");
    assert_eq!(scan.files.len(), 4);
    assert_eq!(
        store
            .project_files(scan.project.id)
            .expect("manifest")
            .len(),
        4
    );
    drop(service);
    let reopened = Arc::new(SqliteStore::open(&database).expect("reopen"));
    let service = ProjectService::new(Arc::clone(&reopened));
    let report = service.import(scan.project.id).expect("import");
    assert_eq!(report.files, 3);
    assert!(report.entities >= 7);
    assert!(
        reopened
            .project_entities(scan.project.id)
            .expect("HTML entities")
            .iter()
            .any(|entity| entity.kind == "html.document")
    );
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

/// Restricts a scan manifest to existing files tracked by Git's index.
#[test]
fn git_tracked_scan_excludes_untracked_files() {
    let root = temporary_directory("git-tracked");
    fs::write(root.join("tracked.md"), "# tracked\n").expect("tracked file");
    fs::write(root.join("untracked.rs"), "fn ignored() {}\n").expect("untracked file");
    git(&root, &["init"]);
    git(&root, &["add", "tracked.md"]);
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let report = ProjectService::new(store)
        .scan_with_options(
            Some("tracked".to_owned()),
            &root,
            ScanOptions { git_tracked: true },
        )
        .expect("tracked scan");
    assert_eq!(
        report
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec!["tracked.md"]
    );
}

/// Rejects Git-filtered scans when the selected path is not a repository.
#[test]
fn git_tracked_scan_requires_repository() {
    let root = temporary_directory("not-git");
    fs::write(root.join("notes.md"), "# notes\n").expect("source file");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let error = ProjectService::new(store)
        .scan_with_options(None, &root, ScanOptions { git_tracked: true })
        .expect_err("non-Git path must fail");
    assert_eq!(error.code, "git_repository_required");
}

/// Imports JavaScript and TypeScript syntax trees and preserves the prior snapshot
/// when a later parse fails.
#[test]
fn ecma_imports_are_atomic_and_diagnostic() {
    let root = temporary_directory("ecma");
    fs::write(
        root.join("main.js"),
        "export function answer() { return 42; }\n",
    )
    .expect("JavaScript");
    fs::write(
        root.join("types.ts"),
        "export interface Answer { value: number }\nexport const answer: Answer = { value: 42 };\n",
    )
    .expect("TypeScript");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan");
    assert_eq!(
        scan.files
            .iter()
            .map(|file| file.adapter.as_str())
            .collect::<Vec<_>>(),
        vec!["javascript", "typescript"]
    );
    let imported = service.import(scan.project.id).expect("ECMAScript import");
    assert!(imported.entities >= 8);

    fs::write(root.join("main.js"), "export function broken( {\n").expect("broken JavaScript");
    service.scan(None, &root).expect("rescan");
    let error = service
        .import(scan.project.id)
        .expect_err("invalid source must reject import");
    assert_eq!(error.code, "import_diagnostics");
    assert_eq!(error.diagnostics[0].adapter, "javascript");
    assert_eq!(
        store
            .project(scan.project.id)
            .expect("project")
            .expect("record")
            .current_snapshot,
        Some(imported.snapshot)
    );
}

/// Imports Python modules and keeps the prior snapshot on syntax failure.
#[test]
fn python_imports_are_atomic_and_diagnostic() {
    let root = temporary_directory("python");
    fs::write(
        root.join("main.py"),
        "# greeting\nclass Greeter:\n    def greet(self, name: str) -> str:\n        return f\"Hi {name}\"\n",
    )
    .expect("Python");
    fs::write(root.join("types.pyi"), "def greet(name: str) -> str: ...\n").expect("Python stub");
    fs::write(root.join("script.pyw"), "print(\"window\")\n").expect("Python script");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan");
    assert_eq!(
        scan.files
            .iter()
            .map(|file| file.adapter.as_str())
            .collect::<Vec<_>>(),
        vec!["python", "python", "python"]
    );
    let imported = service.import(scan.project.id).expect("Python import");
    assert!(imported.entities >= 20);
    assert!(
        store
            .project_entities(scan.project.id)
            .expect("Python entities")
            .iter()
            .any(|entity| entity.kind == "python.document")
    );

    fs::write(root.join("main.py"), "def broken(:\n").expect("broken Python");
    service.scan(None, &root).expect("rescan");
    let error = service
        .import(scan.project.id)
        .expect_err("invalid Python must reject import");
    assert_eq!(error.code, "import_diagnostics");
    assert_eq!(error.diagnostics[0].adapter, "python");
    assert_eq!(
        store
            .project(scan.project.id)
            .expect("project")
            .expect("record")
            .current_snapshot,
        Some(imported.snapshot)
    );
}

/// Imports C and C++ sources and keeps the prior snapshot on syntax failure.
#[test]
fn c_and_cpp_imports_are_atomic_and_diagnostic() {
    let root = temporary_directory("c-family");
    fs::write(
        root.join("main.c"),
        "#include <stdio.h>\nstruct Point { int x; };\nint main(void) { return 0; }\n",
    )
    .expect("C");
    fs::write(root.join("legacy.h"), "int legacy_value;\n").expect("C header");
    fs::write(
        root.join("box.cpp"),
        "namespace demo { template <typename T> class Box { T value; }; }\n",
    )
    .expect("C++");
    fs::write(
        root.join("box.hpp"),
        "class HeaderBox { public: int value; };\n",
    )
    .expect("C++ header");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(None, &root).expect("scan");
    assert_eq!(
        scan.files
            .iter()
            .map(|file| file.adapter.as_str())
            .collect::<Vec<_>>(),
        vec!["cpp", "cpp", "c", "c"]
    );
    let imported = service.import(scan.project.id).expect("C-family import");
    assert!(imported.entities >= 25);
    assert!(
        store
            .project_entities(scan.project.id)
            .expect("C-family entities")
            .iter()
            .any(|entity| entity.kind == "c.document")
    );
    assert!(
        store
            .project_entities(scan.project.id)
            .expect("C++ entities")
            .iter()
            .any(|entity| entity.kind == "cpp.document")
    );

    fs::write(root.join("main.c"), "int broken( {\n").expect("broken C");
    service.scan(None, &root).expect("rescan");
    let error = service
        .import(scan.project.id)
        .expect_err("invalid C must reject import");
    assert_eq!(error.code, "import_diagnostics");
    assert_eq!(error.diagnostics[0].adapter, "c");
    assert_eq!(
        store
            .project(scan.project.id)
            .expect("project")
            .expect("record")
            .current_snapshot,
        Some(imported.snapshot)
    );
}
