//! Catalog merge behaviour for one authorized isolated plugin worker.
//!
//! A worker tool enters a branch tool set only through a registered descriptor
//! that names an explicit plugin capability, and the registration can never
//! displace a trusted semantic tool name. These tests use a real worker process
//! speaking the pinned `rpi-plugin-worker-v1` protocol.

use std::collections::BTreeSet;
use std::sync::Arc;

use rpi_agent::AgentTool as _;
use rpi_extensions::isolated::{HostProcessLauncher, WorkerSandboxPolicy};
use scriberig_agent_runtime::orchestration::ActivitySequence;
use scriberig_agent_runtime::plugin_worker::{
    AuthorizedPluginWorker, PluginToolBridge, PluginToolRequest, sandboxed_launcher,
};
use scriberig_app::{CapabilityContext, CapabilityId};
use scriberig_core::EntityId;

use super::fixture::{
    ECHO_TOOL, FixtureLauncher, FixtureRoot, INSPECT_CAPABILITY, NoOperationDispatcher,
    PLUGIN_CAPABILITY, REPORT_TOOL, SEMANTIC_TOOL, base_catalog, launch_spec, sandbox_policy,
    start_worker,
};

/// Verifies the merged catalog advertises a worker tool only when that tool is
/// both host-authorized and granted to the branch, and hides it otherwise.
#[test]
fn merged_catalog_advertises_only_authorized_granted_worker_tools() {
    let root = FixtureRoot::new("catalog-merge");
    let worker = Arc::new(
        start_worker(&[ECHO_TOOL, REPORT_TOOL], &[ECHO_TOOL], &root, None, true)
            .expect("authorized fixture worker"),
    );
    let bridge =
        PluginToolBridge::new(Arc::clone(&worker), PLUGIN_CAPABILITY).expect("attested bridge");

    let granted = CapabilityContext::new([
        CapabilityId::new(PLUGIN_CAPABILITY).expect("plugin capability"),
        CapabilityId::new(INSPECT_CAPABILITY).expect("inspect capability"),
    ]);
    let composed = bridge
        .compose(request(granted.clone()))
        .expect("plugin tools compose");

    assert_eq!(
        composed
            .tools()
            .iter()
            .map(|tool| tool.schema().name.clone())
            .collect::<Vec<_>>(),
        vec![ECHO_TOOL.to_owned()],
        "only the advertised tool the host allowlisted is adapted"
    );
    assert_eq!(
        composed
            .catalog()
            .descriptors_for(&granted)
            .iter()
            .map(|descriptor| descriptor.definition.name.clone())
            .collect::<Vec<_>>(),
        vec![SEMANTIC_TOOL.to_owned(), ECHO_TOOL.to_owned()],
        "the trusted semantic tool keeps its place and the plugin tool joins it"
    );
    assert!(
        composed
            .catalog()
            .authorize_for_context(REPORT_TOOL, &granted)
            .is_err(),
        "an advertised but unallowlisted worker tool never becomes selectable"
    );
    assert!(
        composed
            .plugin_descriptors()
            .iter()
            .all(|descriptor| descriptor.required_capability.as_deref() == Some(PLUGIN_CAPABILITY)),
        "every merged plugin descriptor names its capability explicitly"
    );

    // Without the plugin capability the same branch sees no plugin tool at all.
    let ungranted =
        CapabilityContext::new(
            [CapabilityId::new(INSPECT_CAPABILITY).expect("inspect capability")],
        );
    let hidden = bridge
        .compose(request(ungranted.clone()))
        .expect("composition hides ungranted plugin tools");
    assert!(hidden.tools().is_empty());
    assert_eq!(
        hidden
            .catalog()
            .descriptors_for(&ungranted)
            .iter()
            .map(|descriptor| descriptor.definition.name.clone())
            .collect::<Vec<_>>(),
        vec![SEMANTIC_TOOL.to_owned()],
        "an ungranted plugin capability stays out of provider-visible context"
    );

    root.remove();
}

/// Verifies a worker tool cannot displace a trusted semantic tool name.
#[test]
fn merge_refuses_a_worker_tool_that_shadows_a_registered_tool() {
    let root = FixtureRoot::new("catalog-shadow");
    let worker = Arc::new(
        start_worker(&[SEMANTIC_TOOL], &[SEMANTIC_TOOL], &root, None, true)
            .expect("authorized fixture worker"),
    );
    let bridge = PluginToolBridge::new(worker, PLUGIN_CAPABILITY).expect("attested bridge");

    let compose = bridge.compose(request(CapabilityContext::new([CapabilityId::new(
        PLUGIN_CAPABILITY,
    )
    .expect("plugin capability")])));
    let Err(error) = compose else {
        panic!("a worker tool must not shadow a registered tool");
    };

    assert!(
        error.to_string().contains("already registered"),
        "shadowing did not fail closed: {error}"
    );

    root.remove();
}

/// Verifies no unattested worker can start, so an unattested composition is
/// unrepresentable instead of a rule the caller has to remember.
#[test]
fn composition_refuses_an_unattested_or_unsandboxed_worker() {
    let root = FixtureRoot::new("catalog-sandbox");
    let spec = launch_spec(&[ECHO_TOOL], &root, None);
    let allowed: BTreeSet<String> = [ECHO_TOOL.to_owned()].into_iter().collect();

    let refused_unattested = start_worker(&[ECHO_TOOL], &[ECHO_TOOL], &root, None, false);
    let Err(unattested) = refused_unattested else {
        panic!("a launcher that claims no OS enforcement must be refused");
    };
    assert!(
        unattested.to_string().contains("does not enforce"),
        "a missing attestation did not fail closed: {unattested}"
    );

    let refused_default = AuthorizedPluginWorker::start(
        &spec,
        allowed.clone(),
        BTreeSet::new(),
        Arc::new(NoOperationDispatcher),
        &HostProcessLauncher,
        sandbox_policy(&root),
        std::slice::from_ref(&root.store_dir),
    );
    let Err(default_launcher) = refused_default else {
        panic!("the pinned default launcher cannot enforce an OS sandbox");
    };
    assert!(
        default_launcher
            .to_string()
            .contains("does not enforce the requested filesystem"),
        "the default upstream launcher must be refused: {default_launcher}"
    );

    let refused_policy = AuthorizedPluginWorker::start(
        &spec,
        allowed,
        BTreeSet::new(),
        Arc::new(NoOperationDispatcher),
        &FixtureLauncher { enforcing: true },
        WorkerSandboxPolicy::default(),
        std::slice::from_ref(&root.store_dir),
    );
    let Err(unisolated) = refused_policy else {
        panic!("an unisolated policy must be refused");
    };
    assert!(
        unisolated
            .to_string()
            .contains("require an OS-sandboxed policy"),
        "an unisolated policy did not fail closed: {unisolated}"
    );

    // The platform launcher ScribeRig selects must itself enforce the policy.
    let selected = sandboxed_launcher().expect("a sandbox launcher for this platform");
    assert!(
        selected.guarantees().enforces(&sandbox_policy(&root)),
        "the selected platform launcher `{}` does not enforce the sandbox policy",
        selected.guarantees().platform
    );

    root.remove();
}

/// Builds one composition request against the shared fixture catalog.
///
/// # Parameters
/// - `capabilities`: Branch capability snapshot to compose under.
///
/// # Returns
/// A request with a fresh activity sequence and a discarding evidence sink.
fn request(capabilities: CapabilityContext) -> PluginToolRequest {
    PluginToolRequest {
        catalog: base_catalog(),
        capabilities,
        branch: EntityId::from_bytes([9; 16]),
        activity: Arc::new(ActivitySequence::default()),
        evidence: Arc::new(|_| {}),
    }
}
