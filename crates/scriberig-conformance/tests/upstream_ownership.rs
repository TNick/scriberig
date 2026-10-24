//! Guards the task-144 deletion ledger and the task-149 upstream-ownership limits.
//!
//! Task 149 removed the superseded local commodity owners (vendored `rpi-tui`,
//! `scriberig-tui-kit`, the Ratatui shell, the local terminal event/keymap
//! modules, and the local provider/loop/plugin ABI). These tests keep the
//! static half of that acceptance enforced instead of re-audited by hand: they
//! read the workspace manifests, `Cargo.lock`, `THIRD_PARTY_NOTICES.md`, and the
//! source tree, and they fail when a removed owner reappears, when a `ScribeRig`
//! crate takes direct terminal or dynamic-loader ownership, or when the pinned
//! upstream revision stops being reproducible. They prove dependency
//! ownership only; the behavioral delegation fixtures in task 149 §§7-13 are
//! separate and stay open.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use toml::Value;

/// Workspace members allowed to consume upstream `rpi-*` runtime crates.
const UPSTREAM_RUNTIME_CONSUMERS: [&str; 5] = [
    "apps/scriberig",
    "crates/scriberig-agent-runtime",
    "crates/scriberig-app",
    "crates/scriberig-server",
    "crates/scriberig-tui",
];

/// Dependency names that would re-establish a deleted local commodity owner.
const FORBIDDEN_DEPENDENCIES: [&str; 5] = [
    "crossterm",
    "libloading",
    "ratatui",
    "rpi-harness",
    "scriberig-tui-kit",
];

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

/// Returns the workspace root directory for this crate.
///
/// # Returns
/// The directory holding `Cargo.toml`, `Cargo.lock`, and the member crates.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Reads one TOML document from the workspace.
///
/// - `path`: Manifest or lockfile to parse.
///
/// # Returns
/// The parsed TOML document.
///
/// # Panics
/// Panics when the file is missing or is not valid TOML.
fn read_toml(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("cannot read {}: {error}", path.display());
    });
    text.parse().unwrap_or_else(|error| {
        panic!("cannot parse {}: {error}", path.display());
    })
}

/// Collects the member directories declared by the workspace manifest.
///
/// - `root`: Workspace root holding the manifest.
///
/// # Returns
/// Every workspace member path relative to the workspace root.
///
/// # Panics
/// Panics when the workspace manifest has no member list.
fn workspace_members(root: &Path) -> Vec<String> {
    let manifest = read_toml(&root.join("Cargo.toml"));
    manifest["workspace"]["members"]
        .as_array()
        .expect("workspace members")
        .iter()
        .map(|member| member.as_str().expect("member path").to_owned())
        .collect()
}

/// Collects every dependency key declared by one manifest.
///
/// Normal dependencies, development dependencies, build dependencies, and
/// target-specific dependency tables are all included so an inherited
/// `workspace = true` request for a forbidden crate is still visible.
///
/// - `manifest`: Cargo manifest to inspect.
///
/// # Returns
/// The declared dependency names.
fn declared_dependencies(manifest: &Path) -> BTreeSet<String> {
    let document = read_toml(manifest);
    let mut names = BTreeSet::new();
    collect_dependency_keys(&document, &mut names);
    if let Some(targets) = document.get("target").and_then(Value::as_table) {
        for target in targets.values().filter_map(Value::as_table) {
            collect_dependency_keys(&Value::Table(target.clone()), &mut names);
        }
    }
    names
}

/// Appends the dependency keys of the standard dependency tables.
///
/// - `table`: Manifest table that may hold dependency tables.
/// - `names`: Mutable collection receiving the declared dependency names.
fn collect_dependency_keys(table: &Value, names: &mut BTreeSet<String>) {
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(deps) = table.get(key).and_then(Value::as_table) {
            names.extend(deps.keys().cloned());
        }
    }
}

/// Returns the single git source revision pinned for the `rpi-*` crates.
///
/// - `root`: Workspace root holding the manifest.
///
/// # Returns
/// The pinned git URL and revision recorded in `[workspace.dependencies]`.
///
/// # Panics
/// Panics when no `rpi-*` git dependency is declared.
fn upstream_pin(root: &Path) -> (String, String) {
    let manifest = read_toml(&root.join("Cargo.toml"));
    let deps = manifest["workspace"]["dependencies"]
        .as_table()
        .expect("workspace dependencies");
    for (name, declaration) in deps {
        if !name.starts_with("rpi-") {
            continue;
        }
        let git = declaration
            .get("git")
            .and_then(Value::as_str)
            .expect("git url");
        let rev = declaration.get("rev").and_then(Value::as_str).expect("rev");
        return (git.to_owned(), rev.to_owned());
    }
    panic!("no rpi-* git dependency is pinned")
}

/// Collects every file below one directory.
///
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

/// Verifies no `ScribeRig` manifest re-declares a deleted commodity owner.
#[test]
fn no_member_declares_a_forbidden_commodity_dependency() {
    let root = workspace_root();
    let mut failures: Vec<String> = Vec::new();
    let workspace_manifest = read_toml(&root.join("Cargo.toml"));
    if let Some(deps) = workspace_manifest["workspace"]["dependencies"].as_table() {
        for name in deps.keys() {
            if FORBIDDEN_DEPENDENCIES.contains(&name.as_str()) {
                failures.push(format!("workspace dependencies declare {name}"));
            }
        }
    }
    for member in workspace_members(&root) {
        let manifest = root.join(&member).join("Cargo.toml");
        for name in declared_dependencies(&manifest) {
            if FORBIDDEN_DEPENDENCIES.contains(&name.as_str()) {
                failures.push(format!("{member} declares {name}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "forbidden commodity owners: {failures:?}"
    );
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

/// Verifies only the active TUI crate consumes the upstream terminal runtime.
#[test]
fn only_the_active_tui_crate_consumes_the_upstream_terminal() {
    let root = workspace_root();
    let mut consumers: Vec<String> = Vec::new();
    for member in workspace_members(&root) {
        let manifest = root.join(&member).join("Cargo.toml");
        if declared_dependencies(&manifest).contains("rpi-tui") {
            consumers.push(member);
        }
    }
    assert_eq!(consumers, vec!["crates/scriberig-tui".to_owned()]);
}

/// Verifies semantic foundation crates stay free of upstream runtime crates.
#[test]
fn semantic_crates_stay_independent_of_upstream_runtime() {
    let root = workspace_root();
    let mut failures: Vec<String> = Vec::new();
    for member in workspace_members(&root) {
        if UPSTREAM_RUNTIME_CONSUMERS.contains(&member.as_str()) {
            continue;
        }
        let manifest = root.join(&member).join("Cargo.toml");
        let upstream: Vec<String> = declared_dependencies(&manifest)
            .into_iter()
            .filter(|name| name.starts_with("rpi-"))
            .collect();
        if !upstream.is_empty() {
            failures.push(format!("{member} consumes upstream crates {upstream:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "dependency direction violations: {failures:?}"
    );
}

/// Verifies the upstream crates stay pinned to one reproducible git revision.
#[test]
fn upstream_crates_are_pinned_to_one_reproducible_git_revision() {
    let root = workspace_root();
    let manifest = read_toml(&root.join("Cargo.toml"));
    assert!(
        manifest.get("patch").is_none() && manifest.get("replace").is_none(),
        "a [patch] or [replace] section replaces the pinned upstream source"
    );
    assert!(
        !root.join(".cargo/config.toml").exists(),
        "a Cargo config override hides how upstream is resolved"
    );
    let (git, rev) = upstream_pin(&root);
    let deps = manifest["workspace"]["dependencies"]
        .as_table()
        .expect("workspace dependencies");
    for (name, declaration) in deps {
        if !name.starts_with("rpi-") {
            continue;
        }
        assert_eq!(
            declaration.get("git").and_then(Value::as_str),
            Some(git.as_str())
        );
        assert_eq!(
            declaration.get("rev").and_then(Value::as_str),
            Some(rev.as_str())
        );
    }
    let lock = read_toml(&root.join("Cargo.lock"));
    for package in lock["package"].as_array().expect("locked packages") {
        let Some(source) = package.get("source").and_then(Value::as_str) else {
            continue;
        };
        if let Some(rest) = source.strip_prefix("git+") {
            assert!(
                rest.contains(rev.as_str()),
                "locked git source {source} is not {rev}"
            );
        }
    }
    for member in workspace_members(&root) {
        let document = read_toml(&root.join(&member).join("Cargo.toml"));
        let name = document["package"]["name"].as_str().expect("package name");
        assert!(
            !name.starts_with("rpi-"),
            "{member} would turn upstream source into a workspace member"
        );
    }
}

/// Verifies the isolated plugin worker stays excluded from every default build.
#[test]
fn isolated_plugin_worker_feature_stays_opt_in() {
    let root = workspace_root();
    let mut failures: Vec<String> = Vec::new();
    for member in workspace_members(&root) {
        let document = read_toml(&root.join(&member).join("Cargo.toml"));
        let Some(features) = document.get("features").and_then(Value::as_table) else {
            continue;
        };
        let Some(default) = features.get("default").and_then(Value::as_array) else {
            continue;
        };
        for entry in default {
            let text = entry.as_str().unwrap_or("");
            if text.contains("plugin") || text.contains("extension") {
                failures.push(format!("{member} enables {text} by default"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "fail-closed plugin gates: {failures:?}"
    );
}

/// Verifies the third-party notice still names the pinned upstream revision.
#[test]
fn third_party_notices_name_the_active_pin() {
    let root = workspace_root();
    let (_, rev) = upstream_pin(&root);
    let notices =
        std::fs::read_to_string(root.join("THIRD_PARTY_NOTICES.md")).expect("third-party notices");
    assert!(
        notices.contains(&rev),
        "THIRD_PARTY_NOTICES.md does not name the pinned revision {rev}"
    );
}
