//! Guards plan SQL behind shared application and session-storage operations.

use std::fs;
use std::path::{Path, PathBuf};

const PRODUCT_CRATES: &[&str] = &[
    "scriberig-agent-runtime",
    "scriberig-app",
    "scriberig-cli",
    "scriberig-client",
    "scriberig-protocol",
    "scriberig-tui",
];

const PRIVATE_PLAN_TABLES: &[&str] = &[
    "plan_node_rollup_index",
    "session_plan_records",
    "session_records",
];

/// Keeps product and runtime crates from depending on or naming plan SQL storage.
#[test]
fn plan_writes_stay_behind_shared_storage_operations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("conformance crate is under the workspace crates directory");
    for package in PRODUCT_CRATES {
        let manifest_path = root.join("crates").join(package).join("Cargo.toml");
        assert_no_direct_sql_dependencies(&manifest_path);
        let source = manifest_path
            .parent()
            .expect("package manifest has a parent")
            .join("src");
        let mut files = Vec::new();
        collect_source_files(&source, &mut files)
            .unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
        assert_no_private_plan_tables(files);
    }
    for product_source in [
        root.join("apps/scriberig/src"),
        root.join("crates/scriberig-server/src"),
        root.join("web/apps"),
        root.join("web/packages"),
    ] {
        let mut files = Vec::new();
        collect_source_files(&product_source, &mut files)
            .unwrap_or_else(|error| panic!("read {}: {error}", product_source.display()));
        assert_no_private_plan_tables(files);
    }
}

/// Rejects production dependencies on SQLite drivers or the session SQLite adapter.
///
/// # Parameters
/// - `manifest_path`: Product crate manifest to inspect.
fn assert_no_direct_sql_dependencies(manifest_path: &Path) {
    let source = fs::read_to_string(manifest_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", manifest_path.display()));
    let manifest: toml::Value = toml::from_str(&source)
        .unwrap_or_else(|error| panic!("parse {}: {error}", manifest_path.display()));
    let dependencies = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .expect("package has a production dependencies table");
    for dependency in ["rusqlite", "scriberig-session-sqlite"] {
        assert!(
            !dependencies.contains_key(dependency),
            "{} has direct production dependency {dependency}",
            manifest_path.display()
        );
    }
}

/// Collects Rust source files beneath one production source directory.
///
/// # Parameters
/// - `directory`: Source directory to scan recursively.
/// - `files`: Destination for discovered Rust source paths.
///
/// # Returns
/// Unit after all Rust source paths have been collected.
///
/// # Errors
/// Returns filesystem errors encountered while traversing the directory.
fn collect_source_files(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, files)?;
        } else if path.extension().is_some_and(is_product_source_extension) {
            files.push(path);
        }
    }
    Ok(())
}

/// Identifies Rust and web source files that can own plan write behavior.
///
/// # Parameters
/// - `extension`: File extension to classify.
///
/// # Returns
/// Whether the extension identifies a Rust or web source file.
fn is_product_source_extension(extension: &std::ffi::OsStr) -> bool {
    ["rs", "ts", "tsx", "js", "jsx"]
        .iter()
        .any(|candidate| extension == *candidate)
}

/// Rejects source files that name a private plan storage table.
///
/// # Parameters
/// - `files`: Product source files to inspect.
fn assert_no_private_plan_tables(files: Vec<PathBuf>) {
    for file in files {
        let contents = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
        for table in PRIVATE_PLAN_TABLES {
            assert!(
                !contents.contains(table),
                "{} names private plan table {table}",
                file.display()
            );
        }
    }
}
