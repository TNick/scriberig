//! `Cargo.lock` graph rules and the task-144 deletion-ledger absence rules.
//!
//! The lock rules are the rename-proof backstop: whatever key or alias a
//! manifest uses, the locked graph still names the real package. The manifest
//! rules in `dependencies` catch the same request before a lock refresh.

use std::path::{Path, PathBuf};

use toml::Value;

use crate::manifest::{read_toml, workspace_root};

/// Locked package names that must never appear, even transitively.
///
/// `crossterm` and `libloading` are legal only as transitive `rpi-*`
/// dependencies, so the direct-edge rule below governs them instead.
const FORBIDDEN_LOCK_PACKAGES: [&str; 3] = ["ratatui", "rpi-harness", "scriberig-tui-kit"];

/// Ledger rows recorded as deleted in task 144 §12.2 that must stay absent.
const DELETED_LOCAL_OWNERS: [&str; 15] = [
    "third_party",
    "crates/scriberig-tui-kit",
    "crates/scriberig-tui/src/terminal.rs",
    "crates/scriberig-tui/src/terminal_tests.rs",
    "crates/scriberig-tui/src/event.rs",
    "crates/scriberig-tui/src/keymap.rs",
    "crates/scriberig-agent-runtime/src/events.rs",
    "crates/scriberig-agent-runtime/src/provider.rs",
    "crates/scriberig-agent-runtime/src/openai_compatible.rs",
    "crates/scriberig-agent-runtime/src/openai_request.rs",
    "crates/scriberig-agent-runtime/src/openai_transport.rs",
    "crates/scriberig-agent-runtime/src/openai_wire.rs",
    "crates/scriberig-agent-runtime/src/openai_cancellation.rs",
    "crates/scriberig-agent-runtime/src/stream.rs",
    "apps/scriberig/src/native_tui_agent/upstream_execution/local_provider.rs",
];

/// Collects every file below one directory.
///
/// # Parameters
/// - `directory`: Directory to walk.
///
/// # Returns
/// Every regular file below `directory`.
///
/// # Panics
/// Panics when the directory cannot be read.
fn source_files(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap_or_else(|error| {
        panic!("cannot read {}: {error}", directory.display());
    }) {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            files.extend(source_files(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// Verifies the locked graph keeps terminal ownership inside `rpi-*` crates.
#[test]
fn locked_graph_keeps_terminal_ownership_upstream() {
    let root = workspace_root();
    let lock = read_toml(&root.join("Cargo.lock"));
    let packages = lock["package"].as_array().expect("locked packages");
    let mut failures: Vec<String> = Vec::new();
    for package in packages {
        let name = package["name"].as_str().expect("locked package name");
        if FORBIDDEN_LOCK_PACKAGES.contains(&name) {
            failures.push(format!("{name} is present in Cargo.lock"));
        }
        let Some(dependencies) = package.get("dependencies").and_then(Value::as_array) else {
            continue;
        };
        for dependency in dependencies {
            let text = dependency.as_str().expect("dependency entry");
            let declared = text.split(' ').next().unwrap_or(text);
            if declared == "crossterm" && !name.starts_with("rpi-") {
                failures.push(format!("{name} depends on crossterm directly"));
            }
        }
    }
    assert!(failures.is_empty(), "locked graph violations: {failures:?}");
}

/// Verifies the ledger's deleted owners stay absent from the source tree.
#[test]
fn deletion_ledger_entries_stay_absent() {
    let root = workspace_root();
    let mut failures: Vec<String> = Vec::new();
    for deleted in DELETED_LOCAL_OWNERS {
        if root.join(deleted).exists() {
            failures.push(format!("{deleted} is back in the tree"));
        }
    }
    for file in source_files(&root.join("crates/scriberig-tui/src")) {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name.starts_with("render") {
            failures.push(format!("{} is a restored renderer owner", file.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "deletion ledger regressions: {failures:?}"
    );
}

/// Verifies the product composes exactly one upstream terminal event loop.
#[test]
fn upstream_tui_owns_one_terminal_render_loop() {
    let root = workspace_root();
    let source_paths = source_files(&root.join("crates/scriberig-tui/src"))
        .into_iter()
        .chain(source_files(&root.join("apps/scriberig/src")));
    let mut runtime_constructions = 0;
    let mut render_loops = 0;
    for path in source_paths {
        let file_stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        if path
            .components()
            .any(|component| component.as_os_str() == "tests")
            || file_stem.ends_with("_tests")
            || file_stem.ends_with("_test")
        {
            continue;
        }
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        runtime_constructions += source.matches("TuiAltScreen::new(").count();
        render_loops += source.matches(".run_until_with_tick(").count();
    }
    assert_eq!(
        runtime_constructions, 1,
        "ScribeRig must compose one upstream terminal runtime"
    );
    assert_eq!(
        render_loops, 1,
        "one upstream lifecycle loop owns terminal rendering and input"
    );
}
