//! Shared fixture for the isolated plugin worker composition boundary.
//!
//! The worker is the real upstream protocol driver running in a separate
//! process; only the OS sandbox launcher is a fixture. It applies the
//! declared-only environment contract and self-reports the restrictions it
//! claims, which is exactly why `ScribeRig` re-checks the launcher attestation
//! instead of trusting the launch configuration.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use rpi_ai::types::Tool;
use rpi_extensions::isolated::{
    WorkerError, WorkerLaunchSpec, WorkerLauncher, WorkerSandboxGuarantees, WorkerSandboxPolicy,
    WorkerSecretPolicy,
};
use scriberig_agent_runtime::plugin_worker::{AuthorizedPluginWorker, PluginSemanticDispatcher};
use scriberig_agent_runtime::tools::{
    ToolCatalog, ToolCategory, ToolDescriptor, ToolExecutionMode,
};
use serde_json::Value;

/// Fixture tool that echoes the worker environment and call arguments.
pub const ECHO_TOOL: &str = "fixture.echo";
/// Fixture tool that returns a fixed result.
pub const REPORT_TOOL: &str = "fixture.report";
/// Fixture tool that waits for a host cancel frame.
pub const WAIT_TOOL: &str = "fixture.wait";
/// Fixture tool that always fails inside the worker.
pub const FAIL_TOOL: &str = "fixture.fail";
/// Plugin capability a branch must hold to see the fixture tools.
pub const PLUGIN_CAPABILITY: &str = "acme.report";
/// Semantic capability required by the trusted catalog entry.
pub const INSPECT_CAPABILITY: &str = "inspect.entity";
/// Trusted semantic tool registered in the base branch catalog.
pub const SEMANTIC_TOOL: &str = "inspect";

/// Launcher that applies the declared-only environment and self-reports its
/// sandbox restrictions.
pub struct FixtureLauncher {
    /// Whether the launcher claims to enforce filesystem, network, and secret
    /// restrictions.
    pub enforcing: bool,
}

impl WorkerLauncher for FixtureLauncher {
    /// Reports the sandbox restrictions this launcher claims to enforce.
    ///
    /// # Returns
    /// The full guarantee set when `enforcing`, and no guarantee otherwise.
    fn guarantees(&self) -> WorkerSandboxGuarantees {
        if self.enforcing {
            return WorkerSandboxGuarantees {
                filesystem: true,
                network: true,
                secrets: true,
                platform: "fixture-sandbox",
            };
        }
        WorkerSandboxGuarantees::none("fixture-unsandboxed")
    }

    /// Builds the worker command with a cleared host environment.
    ///
    /// # Parameters
    /// - `spec`: Host-selected executable, declared environment, and policy.
    ///
    /// # Returns
    /// A command that receives only the declared worker environment.
    ///
    /// # Errors
    /// Returns an error when a sandboxed request asks for an inherited host
    /// environment.
    fn command(&self, spec: &WorkerLaunchSpec) -> io::Result<Command> {
        if spec.sandbox.is_sandboxed() && spec.inherit_environment {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "fixture sandbox cannot inherit the host environment",
            ));
        }
        let mut command = Command::new(&spec.executable);
        command.env_clear().args(&spec.args);
        if spec.sandbox.secrets == WorkerSecretPolicy::DeclaredOnly {
            for (name, value) in &spec.environment {
                command.env(name, value);
            }
        }
        if let Some(directory) = &spec.working_directory {
            command.current_dir(directory);
        }
        Ok(command)
    }
}

/// Host dispatcher that admits no worker semantic operation.
pub struct NoOperationDispatcher;

impl PluginSemanticDispatcher for NoOperationDispatcher {
    /// Denies every worker host call.
    ///
    /// # Parameters
    /// - `operation`: Requested operation identifier.
    /// - `_arguments`: Requested arguments; this fixture denies before reading
    ///   them.
    ///
    /// # Returns
    /// A bounded result; this fixture never produces one.
    ///
    /// # Errors
    /// Returns a safe denial for every operation.
    fn dispatch(&self, operation: &str, _arguments: &Value) -> Result<Value, String> {
        Err(format!("fixture host operation denied: {operation}"))
    }
}

/// Temporary directories that separate plugin state from protected state.
pub struct FixtureRoot {
    /// Root holding the plugin and protected directories.
    pub root: PathBuf,
    /// Directory the worker may read and write.
    pub plugin_dir: PathBuf,
    /// Canonical-store stand-in the worker must never reach.
    pub store_dir: PathBuf,
}

impl FixtureRoot {
    /// Creates one isolated fixture root.
    ///
    /// # Parameters
    /// - `label`: Distinct directory name for this test.
    ///
    /// # Returns
    /// The created fixture root.
    pub fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!("scriberig-plugin-{label}"));
        let _ = std::fs::remove_dir_all(&root);
        let plugin_dir = root.join("plugin");
        let store_dir = root.join("canonical-store");
        std::fs::create_dir_all(&plugin_dir).expect("fixture plugin directory");
        std::fs::create_dir_all(&store_dir).expect("fixture store directory");
        std::fs::write(store_dir.join("scriberig.sqlite"), b"canonical state")
            .expect("fixture store file");
        Self {
            root,
            plugin_dir,
            store_dir,
        }
    }

    /// Directory holding the fixture worker executable.
    ///
    /// # Returns
    /// The absolute directory the sandboxed worker executable lives in.
    pub fn executable_dir() -> PathBuf {
        std::env::current_exe()
            .expect("fixture executable")
            .parent()
            .expect("fixture executable directory")
            .to_path_buf()
    }

    /// Removes the fixture root.
    pub fn remove(&self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Builds the sandbox policy that keeps the protected store out of the worker.
///
/// # Parameters
/// - `root`: Fixture root supplying the plugin and protected directories.
///
/// # Returns
/// A sandboxed policy with the plugin write directory only.
pub fn sandbox_policy(root: &FixtureRoot) -> WorkerSandboxPolicy {
    WorkerSandboxPolicy::sandboxed(
        vec![root.plugin_dir.clone(), FixtureRoot::executable_dir()],
        vec![root.plugin_dir.clone()],
    )
}

/// Builds one sandboxed launch spec for the fixture worker.
///
/// # Parameters
/// - `advertised`: Tool identifiers the worker process advertises.
/// - `root`: Fixture root supplying the plugin write directory.
/// - `marker`: Optional cancellation marker path inside the plugin directory.
///
/// # Returns
/// A declared-only sandboxed launch specification.
pub fn launch_spec(
    advertised: &[&str],
    root: &FixtureRoot,
    marker: Option<&Path>,
) -> WorkerLaunchSpec {
    let mut environment = BTreeMap::new();
    environment.insert("SCRIG_FIXTURE_TOOLS".to_owned(), advertised.join(","));
    if let Some(marker) = marker {
        environment.insert(
            "SCRIG_FIXTURE_CANCEL_MARKER".to_owned(),
            marker.display().to_string(),
        );
    }
    WorkerLaunchSpec {
        executable: PathBuf::from(env!("CARGO_BIN_EXE_plugin-worker-fixture")),
        environment,
        working_directory: Some(root.plugin_dir.clone()),
        request_timeout: Duration::from_secs(5),
        sandbox: sandbox_policy(root),
        ..WorkerLaunchSpec::default()
    }
}

/// Starts one authorized fixture worker.
///
/// # Parameters
/// - `advertised`: Tool identifiers the worker process advertises.
/// - `allowed`: Tool identifiers the host authorizes.
/// - `root`: Fixture root supplying the plugin directories.
/// - `marker`: Optional cancellation marker path.
/// - `enforcing`: Whether the launcher claims OS enforcement.
///
/// # Returns
/// The authorized worker.
///
/// # Errors
/// Returns the sandbox, startup, or protocol failure.
pub fn start_worker(
    advertised: &[&str],
    allowed: &[&str],
    root: &FixtureRoot,
    marker: Option<&Path>,
    enforcing: bool,
) -> Result<AuthorizedPluginWorker, WorkerError> {
    let spec = launch_spec(advertised, root, marker);
    let launcher = FixtureLauncher { enforcing };
    AuthorizedPluginWorker::start(
        &spec,
        allowed.iter().map(|name| (*name).to_owned()).collect(),
        BTreeSet::new(),
        Arc::new(NoOperationDispatcher),
        &launcher,
        sandbox_policy(root),
        std::slice::from_ref(&root.store_dir),
    )
}

/// Builds the trusted base branch catalog containing one semantic tool.
///
/// # Returns
/// A catalog whose only entry is the trusted semantic descriptor.
pub fn base_catalog() -> ToolCatalog {
    ToolCatalog::new(vec![ToolDescriptor {
        definition: Tool {
            name: SEMANTIC_TOOL.to_owned(),
            description: "Inspect semantic state".to_owned(),
            parameters: serde_json::json!({"type": "object"}).into(),
            constrained_sampling: None,
        },
        category: ToolCategory::SemanticRead,
        result_shape: "compact semantic result".to_owned(),
        invocation_label: None,
        state_changing: false,
        execution_mode: ToolExecutionMode::Atomic,
        required_capability: Some(INSPECT_CAPABILITY.to_owned()),
    }])
}
