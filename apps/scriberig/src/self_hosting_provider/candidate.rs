//! Builds the scripted candidate through provider-neutral effect data.

use super::SemanticScope;
use super::tool_call;
use scriberig_agent_runtime::provider::ModelResponse;
use serde_json::{Value, json};

/// Builds the project-wide PHP candidate from checked-in source-reference data.
///
/// # Parameters
/// - `scope`: Exact project and immutable base snapshot.
/// - `validation_failure`: Whether to propose an intentionally invalid Rust crate root.
/// - `super`: Parent scenario module.
///
/// # Returns
/// A normal project proposal tool call containing source effects.
///
/// # Errors
/// Returns serialization failures as provider errors.
pub(super) fn project_proposal(scope: &SemanticScope, validation_failure: bool) -> ModelResponse {
    let effects = candidate_effects(validation_failure);
    tool_call(
        "project_propose",
        json!({"project":scope.project,"base_snapshot":scope.snapshot,"effects":effects}),
    )
}

/// Loads exact source texts from the deterministic self-hosting reference fixture.
///
/// # Parameters
/// - `validation_failure`: Whether to replace the crate root with a compile error.
///
/// # Returns
/// Ordered project-relative create/update effects.
fn candidate_effects(validation_failure: bool) -> Vec<Value> {
    let mut effects = vec![
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
        effect(
            "create",
            "crates/scriberig-project/tests/php_ingestion.rs",
            include_str!(
                "../../../../tests/self-hosting/php-extension/candidate/changes/crates/scriberig-project/tests/php_ingestion.rs"
            ),
        ),
    ];
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

/// Encodes one UTF-8 candidate source effect.
///
/// # Parameters
/// - `kind`: Candidate create/update operation.
/// - `path`: Project-relative source path.
/// - `content`: Exact proposed source content.
///
/// # Returns
/// Provider-neutral structured effect value.
fn effect(kind: &str, path: &str, content: &str) -> Value {
    json!({"kind":kind,"path":path,"content":content})
}
