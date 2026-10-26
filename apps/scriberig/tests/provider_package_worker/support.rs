//! Shares provider-worker fixture helpers across integration scenarios.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::DenyOperations;
use rpi_ai::{Content, Context, Provider, SimpleStreamOptions};
use rpi_extensions::isolated::{WorkerLaunchSpec, WorkerLauncher};
use scriberig_agent_runtime::plugin_worker::{
    AuthorizedPluginWorker, PluginSemanticDispatcher, WorkerNetworkPolicy, WorkerSandboxPolicy,
};

/// Returns the explicit provider allowlist used by the fixture host.
///
/// # Returns
/// The one provider identity the test permits the worker to advertise.
pub(super) fn allowlisted_provider() -> BTreeSet<String> {
    BTreeSet::from(["scriberig-fixture-provider".to_owned()])
}

/// Starts the fixture package in a real product worker process.
///
/// # Parameters
/// - `plugin`: Native fixture artifact loaded by the worker.
/// - `protected_roots`: Host paths the sandbox must keep outside worker access.
/// - `launcher`: Runtime-verified OS sandbox launcher.
/// - `allowed_tools`: Exact tool names permitted by host policy.
///
/// # Returns
/// The admitted isolated worker retaining its plugin library.
///
/// # Panics
/// Panics when worker startup or sandbox admission fails.
pub(super) fn start_fixture_worker(
    plugin: &Path,
    protected_roots: &[PathBuf],
    launcher: &dyn WorkerLauncher,
    allowed_tools: BTreeSet<String>,
) -> Arc<AuthorizedPluginWorker> {
    start_fixture_worker_with_policy(
        plugin,
        protected_roots,
        launcher,
        allowed_tools,
        BTreeSet::new(),
        Arc::new(DenyOperations),
    )
}

/// Starts the fixture with the exact host operation policy selected by a test.
///
/// # Parameters
/// - `plugin`: Native fixture artifact loaded by the worker.
/// - `protected_roots`: Host paths the sandbox must keep outside worker access.
/// - `launcher`: Runtime-verified OS sandbox launcher.
/// - `allowed_tools`: Exact tool names permitted by host policy.
/// - `allowed_operations`: Exact host capabilities permitted by worker policy.
/// - `dispatcher`: `ScribeRig` application operation adapter.
///
/// # Returns
/// The admitted isolated worker retaining its plugin library.
///
/// # Panics
/// Panics when worker startup or sandbox admission fails.
pub(super) fn start_fixture_worker_with_policy(
    plugin: &Path,
    protected_roots: &[PathBuf],
    launcher: &dyn WorkerLauncher,
    allowed_tools: BTreeSet<String>,
    allowed_operations: BTreeSet<String>,
    dispatcher: Arc<dyn PluginSemanticDispatcher>,
) -> Arc<AuthorizedPluginWorker> {
    start_fixture_worker_with_policy_and_commands(
        plugin,
        protected_roots,
        launcher,
        allowed_tools,
        BTreeSet::new(),
        allowed_operations,
        dispatcher,
        BTreeMap::new(),
    )
}

/// Starts the fixture worker with separate tool and command allowlists.
///
/// # Parameters
/// - `plugin`: Native fixture artifact loaded by the worker.
/// - `protected_roots`: Host paths the sandbox must keep outside worker access.
/// - `launcher`: Runtime-verified OS sandbox launcher.
/// - `allowed_tools`: Exact tool names permitted by host policy.
/// - `allowed_commands`: Exact command names permitted by host policy.
///
/// # Returns
/// The admitted isolated worker retaining its plugin library.
///
/// # Panics
/// Panics when worker startup or sandbox admission fails.
pub(super) fn start_fixture_worker_with_commands(
    plugin: &Path,
    protected_roots: &[PathBuf],
    launcher: &dyn WorkerLauncher,
    allowed_tools: BTreeSet<String>,
    allowed_commands: BTreeSet<String>,
) -> Arc<AuthorizedPluginWorker> {
    start_fixture_worker_with_policy_and_commands(
        plugin,
        protected_roots,
        launcher,
        allowed_tools,
        allowed_commands,
        BTreeSet::new(),
        Arc::new(DenyOperations),
        BTreeMap::new(),
    )
}

/// Starts a provider fixture with test-owned environment and host operations.
///
/// # Parameters
/// - `plugin`: Native fixture artifact loaded by the worker.
/// - `protected_roots`: Canonical paths the worker must not access.
/// - `launcher`: Runtime-verified OS sandbox launcher.
/// - `allowed_operations`: Host capabilities the worker may request.
/// - `dispatcher`: Test host operation dispatcher.
/// - `environment`: Explicit environment values passed to the fixture worker.
///
/// # Returns
/// The admitted isolated worker.
///
/// # Panics
/// Panics when worker startup or sandbox admission fails.
pub(super) fn start_fixture_worker_with_environment(
    plugin: &Path,
    protected_roots: &[PathBuf],
    launcher: &dyn WorkerLauncher,
    allowed_operations: BTreeSet<String>,
    dispatcher: Arc<dyn PluginSemanticDispatcher>,
    environment: BTreeMap<String, String>,
) -> Arc<AuthorizedPluginWorker> {
    start_fixture_worker_with_policy_and_commands(
        plugin,
        protected_roots,
        launcher,
        BTreeSet::new(),
        BTreeSet::new(),
        allowed_operations,
        dispatcher,
        environment,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "worker fixture preserves distinct security policy inputs"
)]
/// Starts a fixture worker with explicit tools, commands, and host operations.
///
/// # Parameters
/// - `plugin`: Native fixture artifact loaded by the worker.
/// - `protected_roots`: Host paths the sandbox must keep outside worker access.
/// - `launcher`: Runtime-verified OS sandbox launcher.
/// - `allowed_tools`: Exact tool names permitted by host policy.
/// - `allowed_commands`: Exact command names permitted by host policy.
/// - `allowed_operations`: Exact host capabilities permitted by worker policy.
/// - `dispatcher`: `ScribeRig` application operation adapter.
/// - `launch_environment`: Environment values supplied to the worker process.
///
/// # Returns
/// The admitted isolated worker retaining its plugin library.
///
/// # Panics
/// Panics when worker startup or sandbox admission fails.
fn start_fixture_worker_with_policy_and_commands(
    plugin: &Path,
    protected_roots: &[PathBuf],
    launcher: &dyn WorkerLauncher,
    allowed_tools: BTreeSet<String>,
    allowed_commands: BTreeSet<String>,
    allowed_operations: BTreeSet<String>,
    dispatcher: Arc<dyn PluginSemanticDispatcher>,
    launch_environment: BTreeMap<String, String>,
) -> Arc<AuthorizedPluginWorker> {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_srig"));
    let mut read_roots = vec![
        PathBuf::from("/usr"),
        PathBuf::from("/bin"),
        PathBuf::from("/lib"),
        PathBuf::from("/lib64"),
        plugin
            .parent()
            .expect("fixture artifact parent")
            .to_path_buf(),
        executable
            .parent()
            .expect("product executable parent")
            .to_path_buf(),
    ];
    read_roots.retain(|root| root.exists());
    let mut environment = BTreeMap::from([(
        "SCRIBE_AGENT_PLUGIN_FILES".to_owned(),
        plugin.to_string_lossy().into_owned(),
    )]);
    environment.extend(launch_environment);
    let launch = WorkerLaunchSpec {
        executable,
        args: vec!["--scriberig-plugin-worker".to_owned()],
        environment,
        inherit_environment: false,
        ..WorkerLaunchSpec::default()
    };
    let mut sandbox = WorkerSandboxPolicy::sandboxed(read_roots, Vec::new());
    sandbox.network = WorkerNetworkPolicy::Deny;
    Arc::new(
        AuthorizedPluginWorker::start_with_commands(
            &launch,
            allowed_tools,
            allowed_commands,
            allowed_operations,
            dispatcher,
            launcher,
            sandbox,
            protected_roots,
        )
        .expect("start the fixture provider in an isolated worker"),
    )
}

/// Invokes one provider adapter and verifies its deterministic response.
///
/// # Parameters
/// - `provider`: Isolated worker adapter to invoke.
/// - `model`: Exact model advertised by the selected generation.
///
/// # Panics
/// Panics when the provider reports an error or returns unexpected content.
pub(super) async fn assert_provider_turn(provider: &dyn Provider, model: &rpi_ai::Model) {
    let events = provider
        .stream_simple(model, &Context::default(), &SimpleStreamOptions::default())
        .await;
    let response = events.result().await.expect("provider response event");
    assert!(
        response.error_message.is_none(),
        "provider returned an error"
    );
    assert!(response.content.iter().any(|content| {
        matches!(content, Content::Text(text) if text.text == "isolated fixture provider completed")
    }));
}

/// Verifies a package generation publishes each allowlisted tool and command once.
///
/// - `worker`: Newly activated package worker.
///
/// # Panics
/// Panics when an expected registration is missing or duplicated.
pub(super) fn assert_refresh_registrations(worker: &AuthorizedPluginWorker) {
    let tools = worker
        .advertised_tools()
        .expect("read refreshed package tools");
    assert_eq!(tools.len(), 1, "package refresh duplicated worker tools");
    assert_eq!(tools[0].name, "scriberig_fixture_tool");
    let commands = worker
        .advertised_commands()
        .expect("read refreshed package commands");
    assert_eq!(commands.len(), 1, "package refresh duplicated commands");
    assert_eq!(commands[0].name, "scriberig-fixture-command");
}

/// Locates the fixture cdylib produced as the test's development dependency.
///
/// # Returns
/// The native provider plugin artifact adjacent to the test executable.
///
/// # Panics
/// Panics when Cargo did not build the fixture cdylib for this target.
pub(super) fn provider_plugin_artifact() -> PathBuf {
    let executable = std::env::current_exe().expect("integration test executable path");
    let deps = executable.parent().expect("test executable directory");
    let filename = if cfg!(target_os = "macos") {
        "libscriberig_test_provider_plugin.dylib"
    } else if cfg!(windows) {
        "scriberig_test_provider_plugin.dll"
    } else {
        "libscriberig_test_provider_plugin.so"
    };
    [
        deps.join(filename),
        deps.parent().expect("profile directory").join(filename),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .unwrap_or_else(|| {
        panic!(
            "provider fixture cdylib not found beside {}",
            deps.display()
        )
    })
}
