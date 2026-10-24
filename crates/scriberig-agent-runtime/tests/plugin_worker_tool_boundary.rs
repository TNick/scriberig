//! Proves a plugin worker's advertised tool cannot enter a default-build branch
//! tool set.
//!
//! Task 146 §11 keeps extension and plugin tools deny-by-default until an
//! isolated worker is activated in production. Two gates keep that boundary
//! closed in every supported build: `SemanticAgentTool::new` adapts only a
//! definition already visible in the branch's `ToolCatalog`, and
//! `ToolCatalog::authorize_for_context` admits only a registered name. The only
//! registration path for a worker tool is `ToolCatalog::merge_authorized`,
//! which exists inside the opt-in `isolated-plugin-worker` composition seam and
//! has no production caller, so a worker tool stays unrepresentable in a default
//! build even when the branch is explicitly granted the plugin's capability
//! name.

use rpi_ai::types::Tool;
use scriberig_agent_runtime::tools::{
    ToolCatalog, ToolCategory, ToolDescriptor, ToolExecutionMode,
};
use scriberig_agent_runtime::upstream_tool::SemanticAgentTool;
use scriberig_app::{CapabilityContext, CapabilityId};
use scriberig_core::EntityId;

/// Capability name a future worker composition would have to grant the branch.
const PLUGIN_CAPABILITY: &str = "acme.report";

/// Tool name an isolated plugin worker advertises for that capability.
const WORKER_TOOL: &str = "acme.report";

/// Verifies a worker-advertised tool is rejected at adapter construction, at
/// provider-selected invocation, and never enters the advertised tool set.
#[test]
fn worker_advertised_tool_cannot_enter_the_branch_tool_set() {
    let catalog = ToolCatalog::new(vec![trusted_descriptor("inspect")]);
    // Maximum privilege: the branch already holds the plugin capability name.
    let capabilities =
        CapabilityContext::new([CapabilityId::new(PLUGIN_CAPABILITY).expect("plugin capability")]);
    let worker_tool = worker_tool(WORKER_TOOL);

    let Err(construction) = SemanticAgentTool::new(
        &worker_tool,
        WORKER_TOOL,
        EntityId::from_bytes([7; 16]),
        catalog.clone(),
        capabilities.clone(),
        |_id, _arguments, _cancellation, _on_update| async move {
            Ok(rpi_agent::AgentToolResult::text("worker dispatch reached"))
        },
    ) else {
        panic!("a worker tool must not adapt into a branch tool")
    };
    assert!(
        construction.contains("not visible to this branch"),
        "worker tool construction did not fail closed: {construction}"
    );

    let selection = catalog
        .authorize_for_context(WORKER_TOOL, &capabilities)
        .expect_err("a worker tool name must not authorize");
    assert!(
        selection.to_string().contains("unknown trusted tool"),
        "worker tool selection did not fail closed: {selection}"
    );

    let visible = catalog.descriptors_for(&CapabilityContext::new([
        CapabilityId::new("inspect.entity").expect("inspect capability"),
        CapabilityId::new(PLUGIN_CAPABILITY).expect("plugin capability"),
    ]));
    assert_eq!(
        visible
            .iter()
            .map(|descriptor| descriptor.definition.name.as_str())
            .collect::<Vec<_>>(),
        vec!["inspect"],
        "catalog registration, not the capability name, decides visibility"
    );

    // Control: the same grant does admit the same tool once it is registered,
    // so the two denials above come from the missing registration seam.
    let mut registered = worker_descriptor(WORKER_TOOL);
    registered.required_capability = Some(PLUGIN_CAPABILITY.to_owned());
    let registered_catalog = ToolCatalog::new(vec![registered]);
    let registered = registered_catalog
        .authorize_for_context(WORKER_TOOL, &capabilities)
        .expect("registered plugin tool is admitted by its grant");
    assert_eq!(registered.definition.name, WORKER_TOOL);
}

/// Creates a trusted semantic descriptor registered in the branch catalog.
///
/// - `name`: Stable semantic tool identifier.
///
/// # Returns
/// A descriptor that the granted capability context exposes.
fn trusted_descriptor(name: &str) -> ToolDescriptor {
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

/// Builds the provider-facing schema for one worker-advertised tool.
///
/// - `name`: Tool name the worker advertises in its `WorkerToolSpec`.
///
/// # Returns
/// A schema shaped like an upstream worker registration.
fn worker_tool(name: &str) -> Tool {
    Tool {
        name: name.to_owned(),
        description: "Report through an isolated plugin worker".to_owned(),
        parameters: serde_json::json!({"type": "object"}).into(),
        constrained_sampling: None,
    }
}

/// Builds the descriptor a future composition would have to register.
///
/// - `name`: Worker-advertised tool name.
///
/// # Returns
/// A descriptor whose required capability is the plugin capability name.
fn worker_descriptor(name: &str) -> ToolDescriptor {
    ToolDescriptor {
        definition: worker_tool(name),
        category: ToolCategory::OpaqueFallback,
        result_shape: "bounded worker result".to_owned(),
        invocation_label: None,
        state_changing: true,
        execution_mode: ToolExecutionMode::Atomic,
        required_capability: None,
    }
}
