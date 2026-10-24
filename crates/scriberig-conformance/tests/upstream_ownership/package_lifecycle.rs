//! Package, extension, and catalog-publication ownership rules for task 145 §9.
//!
//! Task 145 keeps provider-package discovery, registry persistence, artifact
//! activation, extension loading, and catalog refresh on the pinned upstream
//! `rpi-*` boundary. The pinned `rpi-extensions` crate owns the project package
//! registry, the project resource lookup, activation of an already-built
//! artifact, uninstall, the generic native-package registry primitives, the
//! reload predicates, and the in-process `ExtensionSession` loader; the pinned
//! `rpi-model-runtime` crate owns atomic catalog publication and the
//! configuration refresh transaction. Nothing in that inventory makes package
//! reload available to `ScribeRig`: the fork has no API that stages a replacement
//! extension session and its model-catalog rows as one generation, publishes
//! them atomically, and keeps the previous generation alive for in-flight
//! calls, so the product gate keeps every native-code input rejected.
//!
//! These rules keep the rejected state from silently turning into a local
//! substitute. They reject a `ScribeRig`-owned package registry, package
//! manifest format, package-directory scan, or in-process loader call, and they
//! confine complete catalog publication to the one shared reload entry that
//! names the upstream generation. They prove source ownership only; the
//! upstream lifecycle gap itself stays recorded in task 145 §9 and §17.6.

use std::path::{Path, PathBuf};

use crate::manifest::workspace_root;

/// Source directories whose contents are product or reusable-crate code.
const OWNED_SOURCE_DIRECTORIES: [&str; 2] = ["crates", "apps"];

/// Directory this guard lives in, excluded from the source scan.
///
/// The rules below name the upstream package and extension APIs they forbid, so
/// scanning this directory would match the rule table itself. Every other
/// workspace source is scanned, and `dependencies` keeps upstream `rpi-*` crates
/// out of this crate, so the exclusion cannot hide a real package owner.
const GUARD_DIRECTORY: &str = "crates/scriberig-conformance/tests/upstream_ownership";

/// Identifiers and wire formats that would make `ScribeRig` a package owner.
///
/// Each name is an upstream-owned package-registry type, registry file,
/// registry operation, provider-manifest type, or package resource lookup in
/// the pinned `rpi-extensions` API. A `ScribeRig` source file that names one is
/// re-establishing the registry, manifest format, or package-directory scan
/// that task 145 §9 assigns upstream.
const PACKAGE_OWNERSHIP_PATTERNS: [&str; 13] = [
    "ProjectPackageRegistry",
    "ProjectPackage",
    "PackageRegistry",
    "packages.json",
    ".rpi/packages",
    "read_project_registry",
    "write_project_registry",
    "project_package_resource_dirs",
    "package_resource_dirs",
    "install_project_artifact",
    "uninstall_project_package",
    "ProviderManifest",
    "provider_manifest",
];

/// Upstream in-process extension-loading surface `ScribeRig` must not consume.
///
/// The pinned loader maps plugin cdylibs into the host address space, which
/// cannot protect canonical `ScribeRig` storage, so task 144 G10 and task 146
/// keep activation deny-by-default. Any consumer of these names would bypass
/// the isolated-worker gate that the product path enforces at
/// `apps/scriberig/src/native_tui_agent/provider_config/extensions.rs`.
const IN_PROCESS_EXTENSION_PATTERNS: [&str; 8] = [
    "ExtensionSession",
    "load_session_mixed",
    "rpi_extensions::load_session",
    "rpi_extensions::load_dir",
    "rpi_extensions::load_one",
    "PluggableProvider",
    "PluginKeepalive",
    "merge_registries",
];

/// Complete-catalog publication call that only the reload contract may use.
const SNAPSHOT_PUBLICATION: &str = "reconcile_models_snapshot";

/// The one production owner allowed to publish a complete catalog snapshot.
const SNAPSHOT_PUBLICATION_OWNER: &str = "crates/scriberig-app/src/provider_lifecycle.rs";

/// Provider-keyed reconciliation that would create a second reload path.
const PROVIDER_KEYED_RECONCILIATION: &str = "reconcile_providers";

/// Collects every Rust source file below one directory.
///
/// - `directory`: Directory to walk.
///
/// # Returns
/// Every regular `.rs` file below `directory`, in directory order.
///
/// # Panics
/// Panics when the directory cannot be read.
fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap_or_else(|error| {
        panic!("cannot read {}: {error}", directory.display());
    }) {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            files.extend(rust_sources(&path));
        } else if path.extension().and_then(|name| name.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files
}

/// Returns the workspace-relative path used in failure messages.
///
/// - `root`: Workspace root holding `crates` and `apps`.
/// - `file`: Source file to name.
///
/// # Returns
/// The path relative to the workspace root, or the absolute path when the file
/// is not below it.
fn relative_path(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string()
}

/// Reads one source file.
///
/// - `file`: Source file to read.
///
/// # Returns
/// The file contents.
///
/// # Panics
/// Panics when the file cannot be read.
fn read_source(file: &Path) -> String {
    std::fs::read_to_string(file)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()))
}

/// Reports whether one workspace-relative source path is test fixture code.
///
/// A fixture may plant upstream-shaped registry data to prove the product
/// ignores it, so the package-ownership rule judges product sources. The
/// in-process extension-host and catalog-publication rules keep no such
/// exception: no source, product or test, may consume the in-process loader or
/// publish a complete catalog snapshot outside the reload contract.
///
/// - `path`: Workspace-relative source path.
///
/// # Returns
/// `true` when the path is a test target, test module, or focused test file.
fn is_test_fixture(path: &str) -> bool {
    let mut parts = path.split('/');
    let file = parts.next_back().unwrap_or("");
    parts.any(|part| part == "tests" || part.ends_with("_tests"))
        || file == "tests.rs"
        || file.ends_with("_tests.rs")
        || file.ends_with("_test.rs")
}

/// Collects every owned source file in the workspace.
///
/// - `product_only`: Whether to skip test targets, test modules, and focused
///   test files.
///
/// # Returns
/// Every `.rs` file under `crates` and `apps` except this guard's own modules,
/// optionally limited to product sources.
fn owned_sources(product_only: bool) -> Vec<(String, String)> {
    let root = workspace_root();
    let mut files = Vec::new();
    for directory in OWNED_SOURCE_DIRECTORIES {
        for file in rust_sources(&root.join(directory)) {
            let relative = relative_path(&root, &file);
            if relative.starts_with(GUARD_DIRECTORY) || (product_only && is_test_fixture(&relative))
            {
                continue;
            }
            files.push((relative, read_source(&file)));
        }
    }
    files
}

/// Collects the ownership patterns one source file names.
///
/// - `path`: Workspace-relative source path, used in the message.
/// - `text`: Source contents.
/// - `patterns`: Names that must not appear.
///
/// # Returns
/// One message per forbidden name the file contains.
fn violations(path: &str, text: &str, patterns: &[&str]) -> Vec<String> {
    patterns
        .iter()
        .filter(|pattern| text.contains(**pattern))
        .map(|pattern| format!("{path} names {pattern}"))
        .collect()
}

/// Verifies no `ScribeRig` product source owns a package registry, manifest
/// format, or package-resource lookup.
#[test]
fn no_source_owns_a_package_registry_or_manifest_format() {
    let failures: Vec<String> = owned_sources(true)
        .iter()
        .flat_map(|(path, text)| violations(path, text, &PACKAGE_OWNERSHIP_PATTERNS))
        .collect();
    assert!(
        failures.is_empty(),
        "local package ownership returned: {failures:?}"
    );
}

/// Verifies no `ScribeRig` source consumes the upstream in-process extension host.
#[test]
fn no_source_consumes_the_in_process_extension_host() {
    let failures: Vec<String> = owned_sources(false)
        .iter()
        .flat_map(|(path, text)| violations(path, text, &IN_PROCESS_EXTENSION_PATTERNS))
        .collect();
    assert!(
        failures.is_empty(),
        "in-process extension consumption returned: {failures:?}"
    );
}

/// Verifies complete catalog publication stays inside the shared reload contract.
///
/// The reload contract in `SNAPSHOT_PUBLICATION_OWNER` is the only production
/// caller of upstream's atomic complete-snapshot publication; the native
/// configuration load and model-property save paths use upstream's
/// `refresh_from_configuration` transaction instead. A second snapshot caller
/// would publish catalog rows outside the named generation.
#[test]
fn one_owner_publishes_each_complete_catalog_snapshot() {
    let mut failures: Vec<String> = Vec::new();
    for (path, text) in owned_sources(false) {
        if text.contains(SNAPSHOT_PUBLICATION) && path != SNAPSHOT_PUBLICATION_OWNER {
            failures.push(format!("{path} publishes a complete catalog snapshot"));
        }
        if text.contains(PROVIDER_KEYED_RECONCILIATION) {
            failures.push(format!(
                "{path} reconciles providers outside the reload contract"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "catalog publication violations: {failures:?}"
    );
}

/// Verifies the product package-lifecycle gate documents its upstream reason.
///
/// The gate is the fail-closed boundary for every configured native-code input;
/// its rejection text must keep naming the upstream lifecycle gap so a caller
/// learns which upstream API is missing instead of being told a local switch is
/// off.
#[test]
fn the_package_root_gate_names_the_upstream_lifecycle_gap() {
    let root = workspace_root();
    let gate = root.join("apps/scriberig/src/native_tui_agent/provider_config/extensions.rs");
    let text = read_source(&gate);
    assert!(
        text.contains("package roots are unsupported until upstream package lifecycle"),
        "the package-root gate no longer names the upstream lifecycle gate: {}",
        gate.display()
    );
}
