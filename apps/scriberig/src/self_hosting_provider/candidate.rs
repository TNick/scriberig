//! Builds the scripted candidate through provider-neutral effect data.

use super::SemanticScope;
use super::tool_call;
use scriberig_agent_runtime::provider::ModelResponse;
use serde_json::{Value, json};

const PHP_WORKSPACE_MEMBER: &str = "  \"crates/domains/scriberig-domain-php\",\n";
const PHP_LOCK_PACKAGES: &str = r#"
[[package]]
name = "scriberig-domain-php"
version = "0.1.0"
dependencies = [
 "serde",
 "tree-sitter",
 "tree-sitter-php",
]

[[package]]
name = "tree-sitter-php"
version = "0.25.1"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "d9779e392d38af4254f48be0e57b7eed4d50ec7a6e55a7e5e2e74df8f8f032d3"
dependencies = [
 "cc",
 "tree-sitter-language",
]
"#;

/// Builds the project-wide PHP candidate from checked-in source-reference data.
///
/// - `scope`: Exact project and immutable base snapshot.
/// - `validation_failure`: Whether to propose an intentionally invalid Rust crate root.
/// - `super`: Parent scenario module.
///
/// A normal project proposal tool call containing source effects.
///
/// Returns serialization failures as provider errors.
/// # Returns
/// The operation result.
pub(super) fn project_proposal(scope: &SemanticScope, validation_failure: bool) -> ModelResponse {
    let effects = candidate_effects(validation_failure);
    tool_call(
        "project_propose",
        json!({"project":scope.project,"base_snapshot":scope.snapshot,"effects":effects}),
    )
}

/// Loads exact source texts from the deterministic self-hosting reference fixture.
///
/// - `validation_failure`: Whether to replace the crate root with a compile error.
///
/// Ordered project-relative create/update effects.
/// # Returns
/// The operation result.
fn candidate_effects(validation_failure: bool) -> Vec<Value> {
    let mut effects = candidate_create_effects();
    effects.extend(candidate_update_effects());
    if validation_failure {
        effects
            .iter_mut()
            .find(|effect| {
                effect.get("path").and_then(Value::as_str)
                    == Some("crates/domains/scriberig-domain-php/src/lib.rs")
            })
            .expect("PHP crate root effect exists")["content"] =
            Value::String("compile_error!(\"deterministic validation failure\");\n".to_owned());
    }
    effects
}

/// Builds create effects for the deterministic PHP candidate fixture.
///
/// Project-relative creates in fixture order.
/// # Returns
/// The ordered source creation effects.
fn candidate_create_effects() -> Vec<Value> {
    vec![
        effect(
            "create",
            "crates/domains/scriberig-domain-php/Cargo.toml",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/domains/scriberig-domain-php/Cargo.toml"
            ),
        ),
        effect(
            "create",
            "crates/domains/scriberig-domain-php/src/lib.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/domains/scriberig-domain-php/src/lib.rs"
            ),
        ),
        effect(
            "create",
            "crates/domains/scriberig-domain-php/src/syntax.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/domains/scriberig-domain-php/src/syntax.rs"
            ),
        ),
        effect(
            "create",
            "crates/domains/scriberig-domain-php/src/declarations.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/domains/scriberig-domain-php/src/declarations.rs"
            ),
        ),
        effect(
            "create",
            "crates/domains/scriberig-domain-php/tests/contract.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/domains/scriberig-domain-php/tests/contract.rs"
            ),
        ),
        effect(
            "create",
            "crates/scriberig-project/src/support_child_entity.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/scriberig-project/src/support_child_entity.rs"
            ),
        ),
        effect(
            "create",
            "crates/scriberig-project/tests/php_ingestion.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/scriberig-project/tests/php_ingestion.rs"
            ),
        ),
    ]
}

/// Builds update effects for the deterministic PHP candidate fixture.
///
/// Project-relative updates in fixture order.
/// # Returns
/// The ordered source update effects.
fn candidate_update_effects() -> Vec<Value> {
    vec![
        effect("update", "Cargo.toml", candidate_workspace_manifest()),
        effect("update", "Cargo.lock", candidate_workspace_lock()),
        effect(
            "update",
            "crates/scriberig-project/Cargo.toml",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/scriberig-project/Cargo.toml"
            ),
        ),
        effect(
            "update",
            "crates/scriberig-project/src/scan.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/scriberig-project/src/scan.rs"
            ),
        ),
        effect(
            "update",
            "crates/scriberig-project/src/adapters.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/scriberig-project/src/adapters.rs"
            ),
        ),
        effect(
            "update",
            "crates/scriberig-project/src/support.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/scriberig-project/src/support.rs"
            ),
        ),
    ]
}

/// Derives the candidate workspace manifest from the exact compiled checkout.
///
/// The current workspace manifest with only the PHP domain member inserted.
/// # Returns
/// The operation result.
fn candidate_workspace_manifest() -> String {
    let manifest = include_str!("../../../../Cargo.toml");
    assert!(
        !manifest.contains(PHP_WORKSPACE_MEMBER),
        "base workspace already contains the PHP member"
    );
    let anchor = "  \"crates/domains/scriberig-domain-python\",\n";
    manifest.replacen(anchor, &format!("{anchor}{PHP_WORKSPACE_MEMBER}"), 1)
}

/// Derives the candidate lockfile from the exact compiled checkout.
///
/// The current workspace lock graph plus the PHP package and dependency edge.
/// # Returns
/// The operation result.
fn candidate_workspace_lock() -> String {
    let mut lock = include_str!("../../../../Cargo.lock").to_owned();
    assert!(
        !lock.contains("name = \"scriberig-domain-php\""),
        "base lock already contains the PHP package"
    );
    let project_marker = "[[package]]\nname = \"scriberig-project\"\n";
    let project_start = lock
        .find(project_marker)
        .expect("scriberig-project lock package exists");
    let project_tail = project_start + project_marker.len();
    let project_end = lock[project_tail..]
        .find("\n[[package]]")
        .map_or(lock.len(), |offset| project_tail + offset);
    let project_block = &lock[project_start..project_end];
    let dependency_anchor = " \"scriberig-domain-python\",\n";
    assert!(
        project_block.contains(dependency_anchor),
        "scriberig-project lock package contains the Python domain dependency"
    );
    let updated_project = project_block.replacen(
        dependency_anchor,
        &format!("{dependency_anchor} \"scriberig-domain-php\",\n"),
        1,
    );
    lock.replace_range(project_start..project_end, &updated_project);
    if !lock.ends_with('\n') {
        lock.push('\n');
    }
    lock.push_str(PHP_LOCK_PACKAGES);
    lock
}

/// Encodes one UTF-8 candidate source effect.
///
/// - `kind`: Candidate create/update operation.
/// - `path`: Project-relative source path.
/// - `content`: Exact proposed source content.
///
/// Provider-neutral structured effect value.
/// # Returns
/// The operation result.
fn effect(kind: &str, path: &str, content: impl Into<String>) -> Value {
    json!({"kind":kind,"path":path,"content":content.into()})
}

#[cfg(test)]
mod tests {
    use super::{candidate_workspace_lock, candidate_workspace_manifest};

    /// Verifies PHP workspace metadata stays based on the current checkout graph.
    #[test]
    fn php_candidate_derives_workspace_metadata_from_current_checkout() {
        let manifest = candidate_workspace_manifest();
        assert!(manifest.contains("crates/scriberig-tui-kit"));
        assert!(manifest.contains("crates/domains/scriberig-domain-php"));
        assert!(manifest.contains("exclude = [\"third_party/rpi-tui/crate\"]"));

        let lock = candidate_workspace_lock();
        assert!(lock.contains("name = \"rpi-tui\""));
        assert!(lock.contains("name = \"scriberig-tui-kit\""));
        assert!(lock.contains("name = \"scriberig-domain-php\""));
        assert!(lock.contains("name = \"tree-sitter-php\""));
        let project = lock
            .split("[[package]]\nname = \"scriberig-project\"\n")
            .nth(1)
            .and_then(|tail| tail.split("\n[[package]]").next())
            .expect("scriberig-project lock package");
        assert!(project.contains("\"scriberig-domain-php\""));
    }
}
