//! Verifies first-class PHP project ingestion through public project APIs.

use std::fs;
use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// PHP-IMPORT-01 through -05 verify classification, exact bytes, ownership, discovery, and render.
#[test]
fn imports_php_bytes_and_discovers_source_declarations() {
    let root = std::env::temp_dir().join(format!("scriberig-php-import-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("source directory");
    let bytes = include_bytes!("../../../tests/self-hosting/php-extension/php-project/src/Greeter.php");
    fs::write(root.join("src/Greeter.php"), bytes).expect("PHP source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(database).expect("database"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service.scan(Some("php-import".to_owned()), &root).expect("scan");
    let file = scan.files.iter().find(|file| file.relative_path == "src/Greeter.php").expect("PHP manifest file");
    assert_eq!(file.adapter, "php");
    let report = service.import(scan.project.id).expect("PHP import");
    let stored = store.project_files(scan.project.id).expect("persisted source");
    assert_eq!(stored.iter().find(|file| file.relative_path == "src/Greeter.php").expect("file").bytes, bytes);
    let symbols = service.symbols(scan.project.id).expect("PHP symbols");
    for name in ["App", "Greeter", "greet", "helper"] {
        let symbol = symbols.iter().find(|symbol| symbol.name.as_deref() == Some(name)).expect(name);
        assert_eq!(symbol.snapshot, report.snapshot);
        assert_eq!(symbol.path, "src/Greeter.php");
        let projection = service.projection_at(scan.project.id, report.snapshot, symbol.entity).expect("source projection");
        assert_eq!(projection.text, bytes);
    }
    assert!(store.project_entities(scan.project.id).expect("ownership").iter().any(|entity| entity.kind == "php.document"));
}

/// PHP-IMPORT-06 keeps a malformed import out of canonical project state.
#[test]
fn malformed_php_does_not_advance_the_canonical_snapshot() {
    let root = std::env::temp_dir().join(format!("scriberig-php-invalid-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::write(root.join("src/Good.php"), "<?php function good() {}\n").expect("valid PHP");
    let store = Arc::new(SqliteStore::open(root.with_extension("sqlite")).expect("database"));
    let service = ProjectService::new(store);
    let scan = service.scan(Some("php-invalid".to_owned()), &root).expect("scan");
    let first = service.import(scan.project.id).expect("initial import");
    fs::write(root.join("src/Broken.php"), "<?php class Broken { function nope( }\n").expect("broken PHP");
    service.scan(None, &root).expect("rescan");
    let error = service.import(scan.project.id).expect_err("malformed import rejected");
    assert_eq!(error.code, "import_diagnostics");
    assert_eq!(service.workspace(scan.project.id).expect("workspace").snapshot, first.snapshot);
}
