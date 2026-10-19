//! Builds the scripted candidate through provider-neutral effect data.

use super::SemanticScope;
use super::tool_call;
use scriberig_agent_runtime::provider::ModelResponse;
use serde_json::{Value, json};

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
        effect(
            "update",
            "Cargo.toml",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/Cargo.toml"
            ),
        ),
        effect(
            "update",
            "Cargo.lock",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/Cargo.lock"
            ),
        ),
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

/// Encodes one UTF-8 candidate source effect.
///
/// - `kind`: Candidate create/update operation.
/// - `path`: Project-relative source path.
/// - `content`: Exact proposed source content.
///
/// Provider-neutral structured effect value.
/// # Returns
/// The operation result.
fn effect(kind: &str, path: &str, content: &str) -> Value {
    json!({"kind":kind,"path":path,"content":content})
}
