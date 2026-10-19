//! Verifies semantic source lookup prefers reopenable source entities.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use scriberig_project::ProjectService;
use scriberig_store_sqlite::SqliteStore;

/// Returns an isolated temporary root for the semantic-search test.
///
/// - `label`: Short test-specific path suffix.
///
/// A unique temporary project directory.
/// # Returns
/// The operation result.
fn temporary_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "scriberig-semantic-search-{label}-{}",
        std::process::id()
    ))
}

/// Prefers the source entity when named Rust symbols share its path.
#[test]
fn semantic_search_returns_source_before_named_children() {
    let root = temporary_root("source-order");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temporary source root");
    fs::write(
        root.join("main.rs"),
        "pub fn first() {}\npub fn second() {}\n",
    )
    .expect("write Rust source");
    let database = root.with_extension("sqlite");
    let store = Arc::new(SqliteStore::open(&database).expect("open project store"));
    let service = ProjectService::new(Arc::clone(&store));
    let scan = service
        .scan(Some("semantic-search".to_owned()), &root)
        .expect("scan source root");
    let imported = service.import(scan.project.id).expect("import source root");

    let matches = service
        .find_symbols_at(scan.project.id, imported.snapshot, "main.rs", 50)
        .expect("search source path");

    assert_eq!(
        matches.first().map(|symbol| symbol.kind.as_str()),
        Some("source")
    );
    drop(store);
    drop(service);
    fs::remove_dir_all(root).expect("remove temporary source root");
    fs::remove_file(database).expect("remove temporary project store");
}
