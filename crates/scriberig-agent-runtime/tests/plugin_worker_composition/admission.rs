//! Host admission through the single plugin composition entry point.
//!
//! `compose_branch_plugin_tools` is the only `ScribeRig` seam that starts a
//! plugin worker and merges its tools into a branch catalog. These cases pin the
//! admission contract at that seam: a verified injected launcher plus a host
//! allowlist composes exactly one branch tool and one authorized call reaches
//! the worker; a name the worker never advertises and an advertised name the
//! host did not allowlist both stay unselectable; and an injected-but-unverified
//! launcher or no launcher at all fails closed before any worker process is
//! started.

use std::collections::BTreeSet;
use std::sync::Arc;

use rpi_agent::{AgentTool as _, TextContentOrImage};
use rpi_extensions::isolated::WorkerLauncher;
use scriberig_agent_runtime::plugin_worker::{
    PluginWorkerAdmission, PluginWorkerConfiguration, compose_branch_plugin_tools,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use super::fixture::{
    ECHO_TOOL, FixtureLauncher, FixtureRoot, NoOperationDispatcher, PLUGIN_CAPABILITY, REPORT_TOOL,
    SEMANTIC_TOOL, launch_spec, sandbox_policy,
};
use super::helpers::{execute, granted, request};

/// Builds one host admission for the fixture worker.
///
/// # Parameters
/// - `advertised`: Tool identifiers the worker process advertises.
/// - `allowed`: Tool identifiers the host allowlist authorizes.
/// - `root`: Fixture root supplying the plugin and protected directories.
/// - `launcher`: Launcher the host injects, or `None` for no injected launcher.
///
/// # Returns
/// One admission carrying only host-supplied launcher, allowlist, and policy.
fn admission<'launcher>(
    advertised: &[&str],
    allowed: &[&str],
    root: &FixtureRoot,
    launcher: Option<&'launcher dyn WorkerLauncher>,
) -> PluginWorkerAdmission<'launcher> {
    PluginWorkerAdmission {
        launcher,
        configuration: PluginWorkerConfiguration {
            allowed_tools: allowed.iter().map(|name| (*name).to_owned()).collect(),
            allowed_operations: BTreeSet::new(),
            launch: launch_spec(advertised, root, None),
        },
        dispatcher: Arc::new(NoOperationDispatcher),
        sandbox: sandbox_policy(root),
        protected_roots: vec![root.store_dir.clone()],
        capability: PLUGIN_CAPABILITY.to_owned(),
    }
}

/// Verifies an admitted worker composes exactly its advertised, allowlisted tool
/// and that an authorized call reaches the worker and settles.
#[test]
fn admitted_worker_tool_is_composed_and_an_authorized_call_settles() {
    let root = FixtureRoot::new("admission-granted");
    let launcher = FixtureLauncher { enforcing: true };
    let capabilities = granted();

    let composed = compose_branch_plugin_tools(
        admission(&[ECHO_TOOL], &[ECHO_TOOL], &root, Some(&launcher)),
        request(capabilities.clone()),
    )
    .expect("a verified injected launcher plus an allowlist composes the worker");

    assert_eq!(
        composed
            .tools()
            .iter()
            .map(|tool| tool.schema().name.clone())
            .collect::<Vec<_>>(),
        vec![ECHO_TOOL.to_owned()],
        "the one advertised and allowlisted tool is the only composed tool"
    );
    assert_eq!(
        composed
            .catalog()
            .descriptors_for(&capabilities)
            .iter()
            .map(|descriptor| descriptor.definition.name.clone())
            .collect::<Vec<_>>(),
        vec![SEMANTIC_TOOL.to_owned(), ECHO_TOOL.to_owned()],
        "the plugin descriptor joins the branch catalog beside the semantic tool"
    );

    let result = execute(
        &composed.tools()[0],
        json!({"note": "admitted call"}),
        CancellationToken::new(),
    )
    .expect("the authorized call reaches the worker");
    let TextContentOrImage::Text(content) = result
        .content
        .first()
        .expect("one provider-visible text content item")
    else {
        panic!("the provider projection must be text");
    };
    assert!(
        content.text.contains("admitted call"),
        "the worker did not receive the authorized call arguments: {}",
        content.text
    );

    root.remove();
}

/// Verifies a tool name the worker never advertises is never registered or
/// selectable, even when the host allowlist names it.
#[test]
fn a_name_the_worker_never_advertises_stays_unselectable() {
    let root = FixtureRoot::new("admission-unknown");
    let launcher = FixtureLauncher { enforcing: true };
    let capabilities = granted();

    let composed = compose_branch_plugin_tools(
        admission(
            &[ECHO_TOOL],
            &[ECHO_TOOL, "acme.invented"],
            &root,
            Some(&launcher),
        ),
        request(capabilities.clone()),
    )
    .expect("an allowlisted name the worker does not offer is simply absent");

    assert_eq!(
        composed
            .plugin_descriptors()
            .iter()
            .map(|descriptor| descriptor.definition.name.clone())
            .collect::<Vec<_>>(),
        vec![ECHO_TOOL.to_owned()],
        "only a tool the worker advertises gets a descriptor"
    );
    assert!(
        composed
            .catalog()
            .authorize_for_context("acme.invented", &capabilities)
            .is_err_and(|error| error.to_string().contains("unknown trusted tool")),
        "a name no worker advertised must not become selectable"
    );

    root.remove();
}

/// Verifies an advertised worker tool the host did not allowlist stays out of
/// the branch catalog and out of provider-visible context.
#[test]
fn an_advertised_tool_the_host_did_not_allowlist_stays_unselectable() {
    let root = FixtureRoot::new("admission-unallowlisted");
    let launcher = FixtureLauncher { enforcing: true };
    let capabilities = granted();

    let composed = compose_branch_plugin_tools(
        admission(
            &[ECHO_TOOL, REPORT_TOOL],
            &[ECHO_TOOL],
            &root,
            Some(&launcher),
        ),
        request(capabilities.clone()),
    )
    .expect("an unallowlisted advertised tool is refused without failing the branch");

    assert_eq!(
        composed
            .tools()
            .iter()
            .map(|tool| tool.schema().name.clone())
            .collect::<Vec<_>>(),
        vec![ECHO_TOOL.to_owned()],
        "host policy, not the worker, decides which advertised tools are composed"
    );
    assert!(
        composed
            .catalog()
            .authorize_for_context(REPORT_TOOL, &capabilities)
            .is_err_and(|error| error.to_string().contains("unknown trusted tool")),
        "an advertised but unallowlisted worker tool must not become selectable"
    );

    root.remove();
}

/// Verifies the entry point fails closed when the host supplies no launcher or
/// supplies one that reports no OS enforcement.
#[test]
fn composition_fails_closed_without_a_verified_launcher() {
    let root = FixtureRoot::new("admission-no-launcher");
    let capabilities = granted();

    let without_launcher = compose_branch_plugin_tools(
        admission(&[ECHO_TOOL], &[ECHO_TOOL], &root, None),
        request(capabilities.clone()),
    );
    let Err(missing) = without_launcher else {
        panic!("composition with no injected launcher must fail closed");
    };
    assert!(
        missing
            .to_string()
            .contains("the host injected no verified plugin worker launcher"),
        "a missing launcher did not fail closed: {missing}"
    );

    let unverified = FixtureLauncher { enforcing: false };
    let with_unverified = compose_branch_plugin_tools(
        admission(&[ECHO_TOOL], &[ECHO_TOOL], &root, Some(&unverified)),
        request(capabilities),
    );
    let Err(unattested) = with_unverified else {
        panic!("a launcher that reports no OS enforcement must be refused");
    };
    assert!(
        unattested.to_string().contains("does not enforce"),
        "an unverified launcher did not fail closed: {unattested}"
    );

    root.remove();
}
