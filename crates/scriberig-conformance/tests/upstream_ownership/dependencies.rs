//! Per-manifest dependency ownership rules and the inheritance probes.
//!
//! The rules consume the resolved package identities from `manifest`, so a
//! forbidden or upstream crate reached through `workspace = true` inheritance
//! or a `package = "..."` alias fails just like a directly written key, and an
//! inherited request the workspace table cannot resolve fails on its own rather
//! than passing as a clean declaration under an allowed key.

use std::collections::BTreeSet;

use toml::Value;

use crate::manifest::{
    Declaration, member_declarations, parse_workspace_dependencies, read_toml,
    unresolved_inheritances, workspace_declarations, workspace_dependency_table, workspace_members,
    workspace_root,
};

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

/// Probe workspace manifest aliasing a forbidden crate under an allowed key.
const PROBE_FORBIDDEN_WORKSPACE: &str = r#"
[workspace.dependencies]
terminal-backend = { package = "crossterm", version = "0.27.0" }
"#;

/// Probe member manifest inheriting that aliased forbidden crate.
const PROBE_FORBIDDEN_MEMBER: &str = r"
[dependencies]
terminal-backend = { workspace = true }
";

/// Probe member manifest inheriting a forbidden key the workspace table lacks.
const PROBE_FORBIDDEN_INHERITED_KEY: &str = r"
[dependencies]
crossterm = { workspace = true }
";

/// Probe workspace manifest declaring the crate the clean member inherits.
const PROBE_CLEAN_WORKSPACE: &str = r#"
[workspace.dependencies]
scriberig-core = { path = "crates/scriberig-core" }
"#;

/// Probe member manifest inheriting an allowed key the workspace table lacks.
const PROBE_UNRESOLVED_MEMBER: &str = r#"
[dependencies]
scratch-helper = { workspace = true }

[dev-dependencies]
scratch-dev-helper = { workspace = true, features = ["test-util"] }
"#;

/// Probe member manifest whose only request is a ScribeRig-owned crate.
const PROBE_CLEAN_MEMBER: &str = r"
[dependencies]
scriberig-core = { workspace = true }
";

/// Probe workspace manifest aliasing the upstream terminal crate.
const PROBE_UPSTREAM_WORKSPACE: &str = r#"
[workspace.dependencies]
terminal-runtime = { package = "rpi-tui", git = "https://github.com/TNick/pi-rust", rev = "0000000000000000000000000000000000000000" }
"#;

/// Probe member manifest inheriting the aliased upstream crate.
const PROBE_UPSTREAM_MEMBER: &str = r"
[dependencies]
terminal-runtime = { workspace = true }
";

/// Collects the forbidden commodity requests resolved from one manifest.
///
/// # Parameters
/// - `owner`: manifest the declarations belong to, used in the message.
/// - `declarations`: resolved dependency declarations to inspect.
///
/// # Returns
/// One message per forbidden package identity, distinguishing an inherited
/// request from a directly written one.
fn forbidden_declarations(owner: &str, declarations: &[Declaration]) -> Vec<String> {
    let mut failures: Vec<String> = Vec::new();
    for declaration in declarations {
        for name in &declaration.names {
            if FORBIDDEN_DEPENDENCIES.contains(&name.as_str()) {
                failures.push(declaration.violation(owner, name));
            }
        }
    }
    failures
}

/// Parses probe manifest text into a TOML document.
///
/// # Parameters
/// - `text`: probe manifest source.
///
/// # Returns
/// The parsed probe document.
fn probe(text: &str) -> Value {
    text.parse().expect("probe manifest")
}

/// Verifies no `ScribeRig` manifest re-declares a deleted commodity owner.
#[test]
fn no_member_declares_a_forbidden_commodity_dependency() {
    let root = workspace_root();
    let workspace_deps = workspace_dependency_table(&root);
    let mut failures = forbidden_declarations(
        "workspace dependency table",
        &workspace_declarations(&workspace_deps),
    );
    for member in workspace_members(&root) {
        let manifest = read_toml(&root.join(&member).join("Cargo.toml"));
        failures.extend(forbidden_declarations(
            &member,
            &member_declarations(&manifest, &workspace_deps),
        ));
    }
    assert!(
        failures.is_empty(),
        "forbidden commodity owners: {failures:?}"
    );
}

/// Verifies only the active TUI crate consumes the upstream terminal runtime.
#[test]
fn only_the_active_tui_crate_consumes_the_upstream_terminal() {
    let root = workspace_root();
    let workspace_deps = workspace_dependency_table(&root);
    let mut consumers: Vec<String> = Vec::new();
    for member in workspace_members(&root) {
        let manifest = read_toml(&root.join(&member).join("Cargo.toml"));
        let declarations = member_declarations(&manifest, &workspace_deps);
        if declarations
            .iter()
            .any(|declaration| declaration.resolves_to("rpi-tui"))
        {
            consumers.push(member);
        }
    }
    assert_eq!(consumers, vec!["crates/scriberig-tui".to_owned()]);
}

/// Verifies semantic foundation crates stay free of upstream runtime crates.
#[test]
fn semantic_crates_stay_independent_of_upstream_runtime() {
    let root = workspace_root();
    let workspace_deps = workspace_dependency_table(&root);
    let mut failures: Vec<String> = Vec::new();
    for member in workspace_members(&root) {
        if UPSTREAM_RUNTIME_CONSUMERS.contains(&member.as_str()) {
            continue;
        }
        let manifest = read_toml(&root.join(&member).join("Cargo.toml"));
        let upstream: BTreeSet<String> = member_declarations(&manifest, &workspace_deps)
            .iter()
            .flat_map(Declaration::upstream_names)
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

/// Proves the forbidden rule rejects a crate reached through workspace inheritance.
///
/// The probe manifests cover the inheritance gaps the key-only scan missed: a
/// forbidden crate hidden behind an allowed key the member inherits, a forbidden
/// key inherited without a matching `[workspace.dependencies]` entry (judged by
/// the written key, so an unresolvable entry cannot slip past), the aliased
/// workspace entry itself, and a clean member manifest that must keep producing
/// no failure.
#[test]
fn inherited_forbidden_dependency_fails_the_guard() {
    let workspace_deps = parse_workspace_dependencies(&probe(PROBE_FORBIDDEN_WORKSPACE));
    let inherited = forbidden_declarations(
        "probe",
        &member_declarations(&probe(PROBE_FORBIDDEN_MEMBER), &workspace_deps),
    );
    assert_eq!(inherited, vec!["probe inherits crossterm".to_owned()]);

    let unresolvable = forbidden_declarations(
        "probe",
        &member_declarations(&probe(PROBE_FORBIDDEN_INHERITED_KEY), &workspace_deps),
    );
    assert_eq!(unresolvable, vec!["probe inherits crossterm".to_owned()]);

    let workspace_level = forbidden_declarations(
        "workspace dependency table",
        &workspace_declarations(&workspace_deps),
    );
    assert_eq!(
        workspace_level,
        vec!["workspace dependency table declares crossterm".to_owned()]
    );

    let clean = forbidden_declarations(
        "probe",
        &member_declarations(&probe(PROBE_CLEAN_MEMBER), &workspace_deps),
    );
    assert!(clean.is_empty(), "clean probe reported {clean:?}");
}

/// Verifies every inherited request in the workspace resolves to an entry.
#[test]
fn every_inherited_dependency_resolves_in_the_workspace_table() {
    let root = workspace_root();
    let workspace_deps = workspace_dependency_table(&root);
    let mut failures = unresolved_inheritances(
        "workspace dependency table",
        &workspace_declarations(&workspace_deps),
    );
    for member in workspace_members(&root) {
        let manifest = read_toml(&root.join(&member).join("Cargo.toml"));
        failures.extend(unresolved_inheritances(
            &member,
            &member_declarations(&manifest, &workspace_deps),
        ));
    }
    assert!(
        failures.is_empty(),
        "unresolved workspace inheritance: {failures:?}"
    );
}

/// Proves an unresolved `workspace = true` request fails even under an allowed key.
///
/// An inherited request the `[workspace.dependencies]` table does not declare
/// carries no resolvable package identity, so scoring it by its written key
/// would accept any key that happens not to be forbidden. The probe covers such
/// an allowed key in both a normal and a dev-dependency table, the same
/// unresolved request on a forbidden key, and a clean member whose inherited key
/// the workspace table does declare.
#[test]
fn unresolved_workspace_inheritance_fails_the_guard() {
    let workspace_deps = parse_workspace_dependencies(&probe(PROBE_FORBIDDEN_WORKSPACE));
    // Set comparison: failure order follows dependency-table iteration, not key order.
    let unresolved: BTreeSet<String> = unresolved_inheritances(
        "probe",
        &member_declarations(&probe(PROBE_UNRESOLVED_MEMBER), &workspace_deps),
    )
    .into_iter()
    .collect();
    assert_eq!(
        unresolved,
        BTreeSet::from([
            "probe inherits scratch-dev-helper from [workspace.dependencies], which does not declare it".to_owned(),
            "probe inherits scratch-helper from [workspace.dependencies], which does not declare it".to_owned(),
        ])
    );

    let forbidden_unresolved = unresolved_inheritances(
        "probe",
        &member_declarations(&probe(PROBE_FORBIDDEN_INHERITED_KEY), &workspace_deps),
    );
    assert_eq!(
        forbidden_unresolved,
        vec![
            "probe inherits crossterm from [workspace.dependencies], which does not declare it"
                .to_owned()
        ]
    );

    let clean_deps = parse_workspace_dependencies(&probe(PROBE_CLEAN_WORKSPACE));
    let clean = member_declarations(&probe(PROBE_CLEAN_MEMBER), &clean_deps);
    let clean_unresolved = unresolved_inheritances("probe", &clean);
    assert!(
        clean_unresolved.is_empty(),
        "clean probe reported {clean_unresolved:?}"
    );
}

/// Proves the upstream-boundary rules see an aliased inherited upstream crate.
#[test]
fn inherited_upstream_dependency_reaches_the_boundary_rules() {
    let workspace_deps = parse_workspace_dependencies(&probe(PROBE_UPSTREAM_WORKSPACE));
    let declarations = member_declarations(&probe(PROBE_UPSTREAM_MEMBER), &workspace_deps);
    assert!(
        declarations
            .iter()
            .any(|declaration| declaration.resolves_to("rpi-tui"))
    );
    let upstream: BTreeSet<String> = declarations
        .iter()
        .flat_map(Declaration::upstream_names)
        .collect();
    assert_eq!(upstream, BTreeSet::from(["rpi-tui".to_owned()]));
}
