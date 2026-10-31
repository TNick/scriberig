//! Shared test helpers for the plugin worker composition cases.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rpi_agent::{AgentError, AgentTool, AgentToolResult};
use rpi_extensions::isolated::{WorkerError, WorkerLaunchSpec};
use scriberig_agent_runtime::orchestration::ActivitySequence;
use scriberig_agent_runtime::plugin_worker::{AuthorizedPluginWorker, PluginToolRequest};
use scriberig_agent_runtime::upstream_tool::SemanticAgentTool;
use scriberig_app::{CapabilityContext, CapabilityId, ToolExecutionEvidence};
use scriberig_core::EntityId;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::fixture::{
    ECHO_TOOL, FixtureLauncher, FixtureRoot, INSPECT_CAPABILITY, NoOperationDispatcher,
    PLUGIN_CAPABILITY, base_catalog,
};

/// Provider-visible tool text bound by `SemanticAgentTool`.
pub const MAX_PROVIDER_RESULT_BYTES: usize = 16_384;

/// Starts the fixture worker with one explicit launch specification.
///
/// # Parameters
/// - `spec`: Launch specification under test.
/// - `root`: Fixture root supplying protected paths.
///
/// # Returns
/// The started worker or the safe start failure.
///
/// # Errors
/// Returns the sandbox, startup, or protocol failure.
pub fn start_with(
    spec: &WorkerLaunchSpec,
    root: &FixtureRoot,
) -> Result<AuthorizedPluginWorker, WorkerError> {
    AuthorizedPluginWorker::start(
        spec,
        [ECHO_TOOL.to_owned()].into_iter().collect(),
        std::collections::BTreeSet::new(),
        Arc::new(NoOperationDispatcher),
        &FixtureLauncher { enforcing: true },
        spec.sandbox.clone(),
        std::slice::from_ref(&root.store_dir),
    )
}

/// Builds one branch capability snapshot, granting the plugin capability only
/// when `with_plugin` is set.
///
/// # Parameters
/// - `with_plugin`: Whether the branch holds the plugin capability.
///
/// # Returns
/// The capability context.
pub fn capability_context(with_plugin: bool) -> CapabilityContext {
    let mut capabilities = vec![CapabilityId::new(INSPECT_CAPABILITY).expect("inspect capability")];
    if with_plugin {
        capabilities.push(CapabilityId::new(PLUGIN_CAPABILITY).expect("plugin capability"));
    }
    CapabilityContext::new(capabilities)
}

/// Returns the granted branch capability snapshot.
///
/// # Returns
/// A context holding both the semantic and plugin capabilities.
pub fn granted() -> CapabilityContext {
    capability_context(true)
}

/// Builds one composition request with a discarding evidence sink.
///
/// # Parameters
/// - `capabilities`: Branch capability snapshot.
///
/// # Returns
/// The composition request.
pub fn request(capabilities: CapabilityContext) -> PluginToolRequest {
    PluginToolRequest {
        catalog: base_catalog(),
        capabilities,
        branch: EntityId::from_bytes([9; 16]),
        activity: Arc::new(ActivitySequence::default()),
        evidence: Arc::new(|_record: ToolExecutionEvidence| {}),
    }
}

/// Builds one composition request that retains its evidence records.
///
/// # Parameters
/// - `capabilities`: Branch capability snapshot.
/// - `records`: Shared evidence record sink.
///
/// # Returns
/// The composition request.
pub fn request_with(
    capabilities: CapabilityContext,
    records: Arc<Mutex<Vec<ToolExecutionEvidence>>>,
) -> PluginToolRequest {
    PluginToolRequest {
        catalog: base_catalog(),
        capabilities,
        branch: EntityId::from_bytes([9; 16]),
        activity: Arc::new(ActivitySequence::default()),
        evidence: Arc::new(move |record| {
            if let Ok(mut records) = records.lock() {
                records.push(record);
            }
        }),
    }
}

/// Runs one composed adapter on a current-thread runtime.
///
/// # Parameters
/// - `tool`: Composed upstream tool adapter.
/// - `arguments`: Provider arguments.
/// - `cancellation`: Branch cancellation token.
///
/// # Returns
/// The settled provider-visible result or the upstream error.
///
/// # Errors
/// Propagates the adapter error.
pub fn execute(
    tool: &Arc<SemanticAgentTool>,
    arguments: Value,
    cancellation: CancellationToken,
) -> Result<AgentToolResult, AgentError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tool runtime");
    let updates: Arc<dyn Fn(rpi_agent::ToolResultPartial) + Send + Sync> = Arc::new(|_| {});
    runtime.block_on(tool.execute("call-1", arguments, cancellation, updates))
}

/// Waits briefly for the worker-side cancellation marker.
///
/// # Parameters
/// - `marker`: Marker path the worker writes when it receives a cancel frame.
///
/// # Returns
/// `true` once the marker exists.
pub fn waited_for_marker(marker: &Path) -> bool {
    for _ in 0..100 {
        if marker.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    marker.exists()
}
