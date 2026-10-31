//! Verifies trusted tool catalog validation before dispatch.

use rpi_ai::types::Tool;
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
    let error = catalog.authorize("invented").expect_err("unknown tool");

    assert!(error.to_string().contains("unknown trusted tool"));
}

/// Verifies a trusted descriptor cannot bypass the caller capability scope.
#[test]
fn required_capability_is_enforced_before_dispatch() {
    let catalog = ToolCatalog::new(vec![ToolDescriptor {
        definition: Tool {
            name: "delegate".to_owned(),
            description: "Delegate focused work".to_owned(),
            parameters: serde_json::json!({"type": "object"}).into(),
            constrained_sampling: None,
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
        .authorize_for_context("delegate", &context)
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
        definition: Tool {
            name: name.to_owned(),
            description: "Inspect semantic state".to_owned(),
            parameters: serde_json::json!({"type": "object"}).into(),
            constrained_sampling: None,
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

/// Verifies a host-authorized addition must name its capability and cannot
/// reuse a registered tool identifier.
#[test]
fn authorized_merge_refuses_capability_free_and_duplicate_tools() {
    let catalog = ToolCatalog::new(vec![descriptor("inspect")]);

    let mut capability_free = descriptor("acme.report");
    capability_free.required_capability = None;
    let refused = catalog
        .merge_authorized(std::slice::from_ref(&capability_free))
        .expect_err("an addition without a capability must be refused");
    assert!(
        refused.to_string().contains("no required capability"),
        "capability-free addition did not fail closed: {refused}"
    );

    let mut shadow = descriptor("inspect");
    shadow.required_capability = Some("acme.report".to_owned());
    let refused = catalog
        .merge_authorized(std::slice::from_ref(&shadow))
        .expect_err("a duplicate tool name must be refused");
    assert!(
        refused.to_string().contains("already registered"),
        "duplicate tool name did not fail closed: {refused}"
    );

    let mut addition = descriptor("acme.report");
    addition.required_capability = Some("acme.report".to_owned());
    let refused = catalog
        .merge_authorized(vec![addition.clone(), addition].as_slice())
        .expect_err("a repeated addition must be refused");
    assert!(
        refused.to_string().contains("already registered"),
        "repeated addition did not fail closed: {refused}"
    );
    assert_eq!(
        catalog
            .descriptors_for(&inspected())
            .iter()
            .map(|entry| entry.definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["inspect"],
        "a refused merge never mutates the base catalog"
    );
}

/// Returns a capability context that grants the trusted inspect capability.
///
/// # Returns
/// A project-scoped context holding `inspect.entity`.
fn inspected() -> CapabilityContext {
    CapabilityContext::with_grants([Capability {
        scope: CapabilityScope::Project,
        actions: BTreeSet::from([CapabilityAction::Inspect]),
        constraints: CapabilityConstraints::unrestricted(),
    }])
}

/// Verifies a merged tool is visible and authorizable only under its named
/// capability, and the base catalog stays independently usable.
#[test]
fn merged_tool_is_authorized_only_under_its_named_capability() {
    let catalog = ToolCatalog::new(vec![descriptor("inspect")]);
    let mut addition = descriptor("acme.report");
    addition.category = ToolCategory::OpaqueFallback;
    addition.required_capability = Some("acme.report".to_owned());
    let merged = catalog
        .merge_authorized(std::slice::from_ref(&addition))
        .expect("capability-scoped addition merges");
    let mut granted = inspected();
    if let Some(entry) = granted.grants.first_mut() {
        entry
            .actions
            .insert(CapabilityAction::Custom("acme.report".to_owned()));
    }

    assert_eq!(
        merged
            .descriptors_for(&granted)
            .iter()
            .map(|entry| entry.definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["inspect", "acme.report"],
        "semantic tools keep priority over an authorized opaque addition"
    );
    assert_eq!(
        merged
            .authorize("acme.report")
            .ok()
            .map(|entry| entry.definition.name.as_str()),
        Some("acme.report")
    );
    assert_eq!(
        catalog
            .descriptors_for(&granted)
            .iter()
            .map(|entry| entry.definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["inspect"],
        "merging produces a new catalog and never widens the shared base one"
    );

    let denied = merged
        .authorize_for_context("acme.report", &CapabilityContext::default())
        .expect_err("an ungranted branch must not invoke the merged tool");
    assert!(
        denied.to_string().contains("required capability denied"),
        "ungranted merged tool did not fail closed: {denied}"
    );
}
