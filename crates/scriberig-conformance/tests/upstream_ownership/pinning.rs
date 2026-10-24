//! Upstream pin reproducibility and fail-closed feature-gate rules.

use toml::Value;

use crate::manifest::{
    read_toml, upstream_pin, workspace_declarations, workspace_dependency_table, workspace_members,
    workspace_root,
};

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
    let workspace_deps = workspace_dependency_table(&root);
    for declaration in workspace_declarations(&workspace_deps) {
        if declaration.upstream_names().is_empty() {
            continue;
        }
        let entry = &workspace_deps[&declaration.key];
        assert_eq!(entry.get("git").and_then(Value::as_str), Some(git.as_str()));
        assert_eq!(entry.get("rev").and_then(Value::as_str), Some(rev.as_str()));
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
