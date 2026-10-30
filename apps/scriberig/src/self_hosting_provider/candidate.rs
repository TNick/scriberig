//! Builds the scripted candidate through provider-neutral effect data.

use super::SemanticScope;
use super::tool_call;
use rpi_ai::{AssistantMessage, Model};
use serde_json::{Value, json};

const PHP_WORKSPACE_MEMBER: &str = "  \"crates/domains/scriberig-domain-php\",\n";
const PHP_DOMAIN_LOCK_PACKAGE: &str = r#"[[package]]
name = "scriberig-domain-php"
version = "0.1.0"
dependencies = [
 "serde",
 "tree-sitter",
 "tree-sitter-php",
]
"#;

const PHP_GRAMMAR_LOCK_PACKAGE: &str = r#"[[package]]
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
/// - `model`: Upstream model producing the project candidate proposal.
/// - `validation_failure`: Whether to propose an intentionally invalid Rust crate root.
///
/// A normal project proposal tool call containing source effects.
///
/// Returns serialization failures as provider errors.
/// # Returns
/// The operation result.
pub(super) fn project_proposal(
    model: &Model,
    scope: &SemanticScope,
    validation_failure: bool,
) -> AssistantMessage {
    let effects = candidate_effects(validation_failure);
    tool_call(
        model,
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
            candidate_project_manifest(),
        ),
        effect(
            "update",
            "crates/scriberig-project/src/scan.rs",
            candidate_scan_module(),
        ),
        effect(
            "update",
            "crates/scriberig-project/src/adapters.rs",
            candidate_adapters_module(),
        ),
        effect(
            "update",
            "crates/scriberig-project/src/support.rs",
            candidate_support_module(),
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

/// Derives the project package manifest from the exact compiled checkout.
///
/// The current project dependencies with only the PHP domain adapter added.
/// # Returns
/// The operation result.
fn candidate_project_manifest() -> String {
    let manifest = include_str!("../../../../crates/scriberig-project/Cargo.toml");
    assert!(
        !manifest.contains("scriberig-domain-php"),
        "base project manifest already contains the PHP dependency"
    );
    let anchor = "scriberig-domain-python = { path = \"../domains/scriberig-domain-python\" }\n";
    assert!(
        manifest.contains(anchor),
        "base project manifest contains the Python domain dependency"
    );
    manifest.replace(
        anchor,
        &format!(
            "scriberig-domain-php = {{ path = \"../domains/scriberig-domain-php\" }}\n{anchor}"
        ),
    )
}

/// Adds PHP classification to the exact current project scanner.
///
/// The current scanner with PHP selected as its own adapter.
/// # Returns
/// The operation result.
fn candidate_scan_module() -> String {
    let source = include_str!("../../../../crates/scriberig-project/src/scan.rs");
    assert!(source.contains("| \"swift\" | \"php\" | \"rb\""));
    let source = source.replace("| \"swift\" | \"php\" | \"rb\"", "| \"swift\" | \"rb\"");
    let anchor = "        Some(\"py\" | \"pyi\" | \"pyw\") => \"python\",\n";
    assert!(!source.contains("Some(\"php\") => \"php\""));
    assert!(
        source.contains(anchor),
        "project scanner has Python classification"
    );
    source.replace(
        anchor,
        &format!("        Some(\"php\") => \"php\",\n{anchor}"),
    )
}

/// Adds the PHP semantic importer to the exact current adapter module.
///
/// The current adapter module with its deterministic PHP importer appended.
/// # Returns
/// The operation result.
fn candidate_adapters_module() -> String {
    let source = include_str!("../../../../crates/scriberig-project/src/adapters.rs");
    let fixture = include_str!(
        "../../../../tests/self-hosting/php-extension/candidate/changes/crates/scriberig-project/src/adapters.rs"
    );
    let marker = "/// Extracts complete PHP syntax and named declarations.";
    let start = fixture
        .find(marker)
        .expect("PHP adapter fixture contains its importer");
    let importer = fixture[start..].trim();
    assert!(!source.contains(marker), "base adapter has no PHP importer");
    assert!(source.contains("use super::add_child;"));
    let source = source.replace(
        "use super::add_child;",
        "use super::{add_child, add_named_child};",
    );
    format!("{}\n\n{importer}\n", source.trim_end())
}

/// Adds PHP dispatch to the exact current semantic import module.
///
/// The current importer module with one PHP adapter dispatch branch.
/// # Returns
/// The operation result.
fn candidate_support_module() -> String {
    let source = include_str!("../../../../crates/scriberig-project/src/support.rs");
    let import_anchor = "import_geopackage, import_html, import_python, import_qgis";
    assert!(
        source.contains(import_anchor),
        "base importer exposes adapter list"
    );
    let source = source.replace(
        import_anchor,
        "import_geopackage, import_html, import_php, import_python, import_qgis",
    );
    let dispatch_anchor = "        \"python\" => {\n";
    assert!(
        source.contains(dispatch_anchor),
        "base importer dispatches Python files"
    );
    source.replace(
        dispatch_anchor,
        "        \"php\" => {\n            import_php(project, file, source, revisions, changes, &mut ownership)?;\n        }\n        \"python\" => {\n",
    )
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
        &format!(" \"scriberig-domain-php\",\n{dependency_anchor}"),
        1,
    );
    lock.replace_range(project_start..project_end, &updated_project);
    let php_domain_anchor = "[[package]]\nname = \"scriberig-domain-python\"\n";
    let php_domain_position = lock
        .find(php_domain_anchor)
        .expect("Python domain lock package exists");
    lock.insert_str(
        php_domain_position,
        &format!("{PHP_DOMAIN_LOCK_PACKAGE}\n\n"),
    );
    let php_grammar_anchor = "[[package]]\nname = \"tree-sitter-python\"\n";
    let php_grammar_position = lock
        .find(php_grammar_anchor)
        .expect("Python grammar lock package exists");
    lock.insert_str(
        php_grammar_position,
        &format!("{PHP_GRAMMAR_LOCK_PACKAGE}\n\n"),
    );
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
    use super::{
        candidate_adapters_module, candidate_project_manifest, candidate_scan_module,
        candidate_support_module, candidate_workspace_lock, candidate_workspace_manifest,
    };

    /// Verifies PHP workspace metadata stays based on the current checkout graph.
    #[test]
    fn php_candidate_derives_workspace_metadata_from_current_checkout() {
        let manifest = candidate_workspace_manifest();
        assert!(!manifest.contains("crates/scriberig-tui-kit"));
        assert!(manifest.contains("crates/domains/scriberig-domain-php"));
        assert!(!manifest.contains("third_party/rpi-tui"));

        let project_manifest = candidate_project_manifest();
        assert!(project_manifest.contains("scriberig-domain-php"));
        assert!(project_manifest.contains("scriberig-domain-qgis"));
        assert!(candidate_scan_module().contains("Some(\"php\") => \"php\""));
        assert!(candidate_adapters_module().contains("fn import_php("));
        assert!(candidate_support_module().contains("\"php\" => {"));

        let lock = candidate_workspace_lock();
        assert!(lock.contains("name = \"rpi-tui\""));
        assert!(!lock.contains("name = \"scriberig-tui-kit\""));
        assert!(lock.contains("name = \"scriberig-domain-php\""));
        assert!(lock.contains("name = \"tree-sitter-php\""));
        assert!(
            lock.find("name = \"scriberig-domain-php\"").unwrap()
                < lock.find("name = \"scriberig-domain-python\"").unwrap()
        );
        assert!(
            lock.find("name = \"tree-sitter-php\"").unwrap()
                < lock.find("name = \"tree-sitter-python\"").unwrap()
        );
        let project = lock
            .split("[[package]]\nname = \"scriberig-project\"\n")
            .nth(1)
            .and_then(|tail| tail.split("\n[[package]]").next())
            .expect("scriberig-project lock package");
        assert!(project.contains("\"scriberig-domain-php\""));
    }
}
