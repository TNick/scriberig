//! Verifies trusted tool catalog validation before dispatch.

use scriberig_agent_runtime::provider::{ToolCall, ToolDefinition};
use scriberig_agent_runtime::tools::{
    ToolCatalog, ToolCategory, ToolDescriptor, ToolExecutionMode,
};
use scriberig_app::{
    Capability, CapabilityAction, CapabilityConstraints, CapabilityContext, CapabilityScope,
};
use std::collections::BTreeSet;

/// Verifies unknown provider-selected tools are rejected by the catalog.
#[test]
fn unknown_tool_id_is_rejected() {
    let catalog = ToolCatalog::new(vec![descriptor("inspect")]);
    let error = catalog
        .validate(&ToolCall {
            id: "call-1".to_owned(),
            name: "invented".to_owned(),
            arguments: serde_json::json!({}),
        })
        .expect_err("unknown tool");

    assert!(error.to_string().contains("unknown trusted tool"));
}

/// Verifies required structured arguments are validated before invocation.
#[test]
fn required_argument_is_validated() {
    let catalog = ToolCatalog::new(vec![descriptor("inspect")]);
    let error = catalog
        .validate(&ToolCall {
            id: "call-1".to_owned(),
            name: "inspect".to_owned(),
            arguments: serde_json::json!({}),
        })
        .expect_err("missing entity");

    assert!(error.to_string().contains("entity"));
}

/// Verifies delegated tools require purpose, prompt, scope, and result shape.
#[test]
fn delegated_tool_arguments_are_validated() {
    let catalog = ToolCatalog::new(vec![descriptor("delegate")]);
    let error = catalog
        .validate(&ToolCall {
            id: "call-1".to_owned(),
            name: "delegate".to_owned(),
            arguments: serde_json::json!({"purpose": "inspect"}),
        })
        .expect_err("missing delegated prompt");

    assert!(error.to_string().contains("capabilities"));
}

/// Verifies a trusted descriptor cannot bypass the caller capability scope.
#[test]
fn required_capability_is_enforced_before_dispatch() {
    let catalog = ToolCatalog::new(vec![ToolDescriptor {
        definition: ToolDefinition {
            name: "delegate".to_owned(),
            description: "Delegate focused work".to_owned(),
            parameters: serde_json::json!({"type": "object"}),
        },
        category: ToolCategory::BranchProducing,
        result_shape: "compact child result".to_owned(),
        invocation_label: None,
        state_changing: false,
        execution_mode: ToolExecutionMode::ChildBranch,
        required_capability: Some("agent.delegate".to_owned()),
    }]);
    let context = CapabilityContext::with_grants([Capability {
        scope: CapabilityScope::Project,
        actions: BTreeSet::from([CapabilityAction::Inspect]),
        constraints: CapabilityConstraints::unrestricted(),
    }]);

    let error = catalog
        .validate_for_context(
            &ToolCall {
                id: "call-1".to_owned(),
                name: "delegate".to_owned(),
                arguments: serde_json::json!({
                    "purpose": "inspect",
                    "prompt": "find it",
                    "capabilities": ["inspect.entity"],
                    "expected_result": "compact result"
                }),
            },
            &context,
        )
        .expect_err("delegation must be denied");

    assert!(error.to_string().contains("agent.delegate"));
}

/// Bounds and sanitizes descriptor labels before client presentation.
#[test]
fn invocation_labels_are_bounded_and_control_free() {
    let mut trusted = descriptor("inspect");
    trusted.invocation_label = Some(format!("Inspect\n{}", "x".repeat(140)));
    trusted.required_capability = None;
    let catalog = ToolCatalog::new(vec![trusted]);
    let label = &catalog.descriptors_for(&CapabilityContext::default())[0].invocation_label;

    assert_eq!(label.as_ref().unwrap().chars().count(), 128);
    assert!(!label.as_ref().unwrap().chars().any(char::is_control));
}

/// Creates the minimal trusted inspect descriptor used by these tests.
///
/// - `name`: Stable descriptor name.
///
/// A deterministic descriptor fixture.
/// # Returns
/// The operation result.
fn descriptor(name: &str) -> ToolDescriptor {
    ToolDescriptor {
        definition: ToolDefinition {
            name: name.to_owned(),
            description: "Inspect semantic state".to_owned(),
            parameters: serde_json::json!({"type": "object"}),
        },
        category: ToolCategory::SemanticRead,
        result_shape: "compact semantic result".to_owned(),
        invocation_label: None,
        state_changing: false,
        execution_mode: ToolExecutionMode::Atomic,
        required_capability: Some("inspect.entity".to_owned()),
    }
}

/// Prefers semantic operations and keeps opaque fallback explicitly authorized.
#[test]
fn semantic_tools_precede_explicit_opaque_fallbacks() {
    let mut semantic = descriptor("code.replace_expression");
    semantic.category = ToolCategory::SemanticMutation;
    semantic.required_capability = None;
    let mut fallback = descriptor("file.replace_raw");
    fallback.category = ToolCategory::OpaqueFallback;
    fallback.required_capability = Some("file.raw_write".to_owned());
    let catalog = ToolCatalog::new(vec![fallback, semantic]);
    let semantic_context = CapabilityContext::default();
    let opaque_context = CapabilityContext::with_grants([Capability {
        scope: CapabilityScope::Project,
        actions: BTreeSet::from([CapabilityAction::Custom("file.raw_write".to_owned())]),
        constraints: CapabilityConstraints::unrestricted(),
    }]);

    let authorized = catalog.descriptors_for(&opaque_context);
    assert_eq!(authorized[0].definition.name, "code.replace_expression");
    assert_eq!(authorized[1].definition.name, "file.replace_raw");
    assert_eq!(catalog.descriptors_for(&semantic_context).len(), 1);
    assert_eq!(
        catalog.descriptors_for(&semantic_context)[0].category,
        ToolCategory::SemanticMutation
    );
}
