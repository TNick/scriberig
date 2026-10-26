//! Denial of unregistered, unallowlisted, and capability-revoked tool names.

use std::sync::Arc;

use scriberig_agent_runtime::plugin_worker::PluginToolBridge;
use scriberig_agent_runtime::upstream_tool::SemanticAgentTool;
use scriberig_core::EntityId;
use serde_json::json;

use super::fixture::{ECHO_TOOL, PLUGIN_CAPABILITY, REPORT_TOOL, base_catalog, start_worker};
use super::helpers::{capability_context, request};

/// Verifies an unregistered, unallowlisted, or capability-revoked tool name is
/// denied before any worker call is dispatched.
#[test]
fn unknown_or_ungranted_tool_names_are_denied_before_dispatch() {
    let root = super::fixture::FixtureRoot::new("denial");
    let worker = Arc::new(
        start_worker(&[ECHO_TOOL, REPORT_TOOL], &[ECHO_TOOL], &root, None, true)
            .expect("authorized fixture worker"),
    );
    let bridge =
        PluginToolBridge::new(Arc::clone(&worker), PLUGIN_CAPABILITY).expect("attested bridge");
    let granted = capability_context(true);

    let composed = bridge.compose(request(granted.clone())).expect("compose");
    assert_eq!(
        composed.tools().len(),
        1,
        "only the host-authorized advertised tool is adapted"
    );

    let invented = composed
        .catalog()
        .authorize_for_context("acme.invented", &granted)
        .expect_err("an unregistered name must not authorize");
    assert!(
        invented.to_string().contains("unknown trusted tool"),
        "unknown name did not fail closed: {invented}"
    );
    let unallowlisted = composed
        .catalog()
        .authorize_for_context(REPORT_TOOL, &granted)
        .expect_err("an unallowlisted worker tool must not authorize");
    assert!(
        unallowlisted.to_string().contains("unknown trusted tool"),
        "unallowlisted name did not fail closed: {unallowlisted}"
    );

    // The worker itself would run this tool; host policy must refuse it.
    let direct = worker
        .call_tool(REPORT_TOOL, json!({}))
        .expect_err("host policy must deny an unallowlisted tool");
    assert!(
        direct.to_string().contains("not authorized"),
        "worker-level denial did not fail closed: {direct}"
    );

    // A registered plugin name is still denied once the branch loses the grant.
    let revoked = capability_context(false);
    let revoked_error = composed
        .catalog()
        .authorize_for_context(ECHO_TOOL, &revoked)
        .expect_err("a revoked plugin capability must deny the registered name");
    assert!(
        revoked_error
            .to_string()
            .contains("required capability denied: acme.report"),
        "revoked capability did not fail closed: {revoked_error}"
    );

    // And an adapter cannot be built for a definition the branch cannot see.
    let unregistered = SemanticAgentTool::new(
        &rpi_ai::types::Tool {
            name: ECHO_TOOL.to_owned(),
            description: "Unregistered worker tool".to_owned(),
            parameters: json!({"type": "object"}).into(),
            constrained_sampling: None,
        },
        ECHO_TOOL,
        EntityId::from_bytes([9; 16]),
        base_catalog(),
        granted,
        |_id, _arguments, _cancellation, _updates| async move {
            Ok(rpi_agent::AgentToolResult::text("worker dispatch reached"))
        },
    );
    let Err(unseeable) = unregistered else {
        panic!("an unregistered definition must not adapt");
    };
    assert!(
        unseeable.contains("not visible to this branch"),
        "unregistered definition did not fail closed: {unseeable}"
    );

    root.remove();
}
