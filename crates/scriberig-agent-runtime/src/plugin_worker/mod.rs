//! Bridges `ScribeRig` host policy to upstream isolated plugin workers.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use rpi_extensions::isolated::{
    AllowlistedHostHandler, IsolatedWorker, WorkerError, WorkerLaunchSpec, WorkerLauncher,
    WorkerSandboxPolicy, WorkerToolSpec,
};
use tokio_util::sync::CancellationToken;

mod composition;
mod dispatcher;
mod sandbox;

pub use composition::{
    ComposedPluginTools, PluginCompositionError, PluginToolBridge, PluginToolRequest,
};
pub use dispatcher::{ApplicationDomainDispatcher, PluginHostPolicy, PluginSemanticDispatcher};
use sandbox::validate_worker_launch;
pub use sandbox::{WorkerSandboxAttestation, sandboxed_launcher};

/// One host-policy-authorized isolated plugin worker.
///
/// The worker is admitted only under a sandboxed policy whose launcher reports
/// every enforced OS restriction, so the retained attestation is a durable
/// statement about the running process rather than a configuration hope.
///
/// # Public members
/// - `sandbox_attestation`: Launcher-attested OS isolation for this process.
/// - `advertised_tools`: Host-authorized tools this worker can execute.
/// - `advertises`: Reports one advertised tool identifier.
/// - `allows`: Reports one host-allowlisted tool identifier.
/// - `call_tool`: Executes one authorized worker tool.
/// - `call_tool_cancellable`: Executes one authorized tool under cancellation.
///
/// # Private members
/// - `worker`: Upstream-owned worker process and IPC state.
/// - `allowed_tools`: Tool identifiers authorized for this branch.
/// - `attestation`: Launcher-reported isolation recorded at startup.
pub struct AuthorizedPluginWorker {
    /// Retained upstream worker process and IPC state.
    worker: IsolatedWorker,
    /// Tool identifiers authorized for this branch.
    allowed_tools: BTreeSet<String>,
    /// Launcher-reported OS restrictions enforced for this worker process.
    attestation: WorkerSandboxAttestation,
}

impl AuthorizedPluginWorker {
    /// Starts a worker and retains only host-authorized tools.
    ///
    /// The launch is rejected before any process starts unless the sandbox
    /// policy is sandboxed, the environment is not inherited, no selected path
    /// reaches a protected path, and the launcher reports that it enforces the
    /// requested filesystem, network, and secret restrictions.
    ///
    /// # Parameters
    /// - `spec`: Host-selected worker executable and environment.
    /// - `allowed_tools`: Tool names the host is willing to invoke.
    /// - `allowed_operations`: Semantic operations the worker may request.
    /// - `dispatcher`: Host semantic operation route.
    /// - `launcher`: Host-injected OS sandbox launcher.
    /// - `sandbox`: Host-selected filesystem, network, and secret restrictions.
    /// - `protected_roots`: Canonical store, secret, and configuration paths the
    ///   worker must not reach.
    ///
    /// # Returns
    /// A running isolated worker with its recorded sandbox attestation.
    ///
    /// # Errors
    /// Returns a sandbox, startup, or protocol error.
    #[allow(clippy::too_many_arguments)] // Keeps one explicit sandboxed start contract.
    pub fn start(
        spec: &WorkerLaunchSpec,
        allowed_tools: BTreeSet<String>,
        allowed_operations: BTreeSet<String>,
        dispatcher: Arc<dyn PluginSemanticDispatcher>,
        launcher: &dyn WorkerLauncher,
        sandbox: WorkerSandboxPolicy,
        protected_roots: &[PathBuf],
    ) -> Result<Self, WorkerError> {
        let mut spec = spec.clone();
        spec.sandbox = sandbox;
        validate_worker_launch(&spec, protected_roots).map_err(WorkerError::Sandbox)?;
        let guarantees = launcher.guarantees();
        let attestation = WorkerSandboxAttestation::from_guarantees(&guarantees);
        if !attestation.enforces_os_isolation() || !guarantees.enforces(&spec.sandbox) {
            return Err(WorkerError::Sandbox(format!(
                "launcher `{}` does not enforce the requested filesystem, network, and secret restrictions",
                guarantees.platform
            )));
        }
        let dispatcher = Arc::new(PluginHostPolicy::new(dispatcher));
        let host = Arc::new(AllowlistedHostHandler::new(allowed_operations, dispatcher));
        let worker = IsolatedWorker::start(&spec, launcher, Some(host))?;
        Ok(Self {
            worker,
            allowed_tools,
            attestation,
        })
    }

    /// Returns tools advertised by the worker and allowed by host policy.
    ///
    /// # Returns
    /// Authorized tool descriptors in worker order.
    ///
    /// # Errors
    /// Returns a sandbox error when the worker process is not OS-isolated.
    pub fn advertised_tools(&self) -> Result<Vec<WorkerToolSpec>, WorkerError> {
        self.require_sandbox()?;
        Ok(self
            .worker
            .tools()
            .iter()
            .filter(|tool| self.allowed_tools.contains(&tool.name))
            .cloned()
            .collect())
    }

    /// Reports whether the worker process advertised one tool name.
    ///
    /// # Parameters
    /// - `tool`: Tool identifier selected by the provider.
    ///
    /// # Returns
    /// `true` when this worker advertises the exact name.
    pub fn advertises(&self, tool: &str) -> bool {
        self.worker
            .tools()
            .iter()
            .any(|tool_spec| tool_spec.name == tool)
    }

    /// Reports whether host policy allows one tool name.
    ///
    /// # Parameters
    /// - `tool`: Tool identifier selected by the provider.
    ///
    /// # Returns
    /// `true` when the host allowlist contains the exact name.
    pub fn allows(&self, tool: &str) -> bool {
        self.allowed_tools.contains(tool)
    }

    /// Returns the launcher-attested OS isolation for this worker.
    ///
    /// # Returns
    /// The guarantee set recorded when the worker started.
    pub fn sandbox_attestation(&self) -> WorkerSandboxAttestation {
        self.attestation
    }

    /// Executes one authorized worker tool.
    ///
    /// # Parameters
    /// - `tool`: Worker-advertised tool name.
    /// - `arguments`: Tool arguments.
    ///
    /// # Returns
    /// The bounded worker result.
    ///
    /// # Errors
    /// Returns a sandbox, authorization, timeout, cancellation, or worker
    /// failure.
    pub fn call_tool(
        &self,
        tool: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, WorkerError> {
        self.call_tool_cancellable(tool, arguments, None)
    }

    /// Executes one authorized worker tool under a branch cancellation token.
    ///
    /// # Parameters
    /// - `tool`: Worker-advertised tool name.
    /// - `arguments`: Tool arguments.
    /// - `cancellation`: Branch-scoped cancellation token, when supplied.
    ///
    /// # Returns
    /// The bounded worker result.
    ///
    /// # Errors
    /// Returns a sandbox, authorization, timeout, cancellation, or worker
    /// failure.
    pub fn call_tool_cancellable(
        &self,
        tool: &str,
        arguments: serde_json::Value,
        cancellation: Option<&CancellationToken>,
    ) -> Result<serde_json::Value, WorkerError> {
        self.require_sandbox()?;
        if !self.allowed_tools.contains(tool) {
            return Err(WorkerError::Unauthorized(tool.to_owned()));
        }
        self.worker
            .call_tool_cancellable(tool, arguments, cancellation)
    }

    /// Fails closed when the running worker is not OS-isolated.
    ///
    /// # Returns
    /// `Ok(())` when the launcher attested every requested restriction.
    ///
    /// # Errors
    /// Returns a sandbox error naming the attested launcher.
    fn require_sandbox(&self) -> Result<(), WorkerError> {
        if self.attestation.enforces_os_isolation() {
            return Ok(());
        }
        Err(WorkerError::Sandbox(format!(
            "plugin worker launcher `{}` did not enforce OS isolation",
            self.attestation.platform
        )))
    }
}

/// Host configuration for an isolated plugin worker.
///
/// # Public members
/// - `allowed_tools`: Worker tool IDs the host may invoke.
/// - `allowed_operations`: Semantic operations the worker may request.
/// - `launch`: Executable and bounded environment for the worker process.
///
/// # Private members
/// None.
pub struct PluginWorkerConfiguration {
    /// Allowed worker tool identifiers.
    pub allowed_tools: BTreeSet<String>,
    /// Allowed worker semantic operation identifiers.
    pub allowed_operations: BTreeSet<String>,
    /// Launch specification without inherited host environment.
    pub launch: WorkerLaunchSpec,
}

impl PluginWorkerConfiguration {
    /// Reads the host's isolated plugin configuration.
    ///
    /// # Returns
    /// `None` when no plugin roots are configured.
    ///
    /// # Errors
    /// Returns a configuration error when roots are configured without a safe
    /// worker invocation.
    pub fn from_environment() -> Result<Option<Self>, String> {
        let directories = path_list("SCRIBE_AGENT_EXTENSION_DIRS");
        let files = path_list("SCRIBE_AGENT_PLUGIN_FILES");
        if directories.is_empty() && files.is_empty() {
            return Ok(None);
        }
        let executable = std::env::var_os("SCRIBE_AGENT_PLUGIN_WORKER")
            .map(PathBuf::from)
            .map_or_else(std::env::current_exe, Ok)
            .map_err(|error| error.to_string())?;
        if !executable.is_file() {
            return Err("configured plugin worker executable is not a file".to_owned());
        }
        let mut environment = std::collections::BTreeMap::new();
        environment.insert(
            "SCRIBE_AGENT_EXTENSION_DIRS".to_owned(),
            join_paths(&directories)?,
        );
        environment.insert("SCRIBE_AGENT_PLUGIN_FILES".to_owned(), join_paths(&files)?);
        let mut launch = WorkerLaunchSpec {
            executable,
            args: vec!["--scriberig-plugin-worker".to_owned()],
            environment,
            working_directory: None,
            inherit_environment: false,
            ..WorkerLaunchSpec::default()
        };
        if let Some(timeout) = std::env::var("SCRIBE_AGENT_PLUGIN_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
        {
            launch.request_timeout = std::time::Duration::from_secs(timeout);
        }
        Ok(Some(Self {
            allowed_tools: name_set("SCRIBE_AGENT_PLUGIN_TOOLS"),
            allowed_operations: name_set("SCRIBE_AGENT_PLUGIN_CAPABILITIES"),
            launch,
        }))
    }

    /// Starts the configured worker.
    ///
    /// # Parameters
    /// - `dispatcher`: Host semantic operation route.
    /// - `launcher`: Host-injected OS sandbox launcher.
    /// - `sandbox`: Host-selected filesystem, network, and secret restrictions.
    /// - `protected_roots`: Canonical store, secret, and configuration paths the
    ///   worker must not reach.
    ///
    /// # Returns
    /// A running isolated worker.
    ///
    /// # Errors
    /// Returns a sandbox, startup, or protocol error.
    pub fn start(
        self,
        dispatcher: Arc<dyn PluginSemanticDispatcher>,
        launcher: &dyn WorkerLauncher,
        sandbox: WorkerSandboxPolicy,
        protected_roots: &[PathBuf],
    ) -> Result<AuthorizedPluginWorker, WorkerError> {
        let Self {
            allowed_tools,
            allowed_operations,
            launch,
        } = self;
        AuthorizedPluginWorker::start(
            &launch,
            allowed_tools,
            allowed_operations,
            dispatcher,
            launcher,
            sandbox,
            protected_roots,
        )
    }
}

/// Runs the isolated worker mode selected by the host launcher.
///
/// # Parameters
/// - `arguments`: Process arguments inspected for the worker marker.
///
/// # Returns
/// `true` after serving the worker protocol.
///
/// # Errors
/// Returns a plugin load or worker protocol error.
pub fn run_worker_mode(arguments: &[String]) -> Result<bool, String> {
    if !arguments
        .iter()
        .any(|argument| argument == "--scriberig-plugin-worker")
    {
        return Ok(false);
    }
    let directories = path_list("SCRIBE_AGENT_EXTENSION_DIRS");
    let files = path_list("SCRIBE_AGENT_PLUGIN_FILES");
    rpi_extensions::isolated::run_worker(&directories, &files)?;
    Ok(true)
}

/// Splits one host path-list environment variable.
///
/// # Parameters
/// - `name`: Environment variable name.
///
/// # Returns
/// Non-empty configured paths.
fn path_list(name: &str) -> Vec<PathBuf> {
    std::env::var_os(name)
        .map(|value| {
            std::env::split_paths(&value)
                .filter(|path| !path.as_os_str().is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Splits one host identifier-list environment variable.
///
/// # Parameters
/// - `name`: Environment variable name.
///
/// # Returns
/// Non-empty identifiers.
fn name_set(name: &str) -> BTreeSet<String> {
    std::env::var(name)
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Joins paths for transfer into the isolated worker environment.
///
/// # Parameters
/// - `paths`: Host-authorized plugin paths.
///
/// # Returns
/// A platform-encoded path-list string.
///
/// # Errors
/// Returns an error when a path cannot be represented as an environment value.
fn join_paths(paths: &[PathBuf]) -> Result<String, String> {
    std::env::join_paths(paths)
        .map(|paths| paths.to_string_lossy().into_owned())
        .map_err(|error| error.to_string())
}
