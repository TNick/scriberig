//! Bridges `ScribeRig` host policy to upstream isolated plugin workers.

use rpi_ai::Provider;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rpi_extensions::isolated::{
    AllowlistedHostHandler, IsolatedWorker, WorkerError, WorkerLaunchSpec, WorkerLauncher,
    WorkerProvider, WorkerToolSpec,
};
pub use rpi_extensions::isolated::{WorkerNetworkPolicy, WorkerSandboxPolicy};
use tokio_util::sync::CancellationToken;

mod components;
mod composition;
#[path = "configuration.rs"]
mod configuration;
mod dispatcher;
#[path = "packages.rs"]
mod packages;
mod sandbox;

pub use components::PluginComponentDispatcher;
pub use composition::{
    ComposedPluginTools, PluginCompositionError, PluginToolBridge, PluginToolRequest,
    PluginWorkerAdmission, compose_branch_plugin_tools,
};
pub use configuration::{PluginWorkerConfiguration, run_worker_mode};
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
/// - `provider_adapters`: Builds authorized upstream Provider adapters.
/// - `advertised_commands`: Returns host-allowlisted slash commands.
/// - `call_command`: Executes one host-allowlisted slash command.
/// - `call_command_action`: Continues an allowlisted command with a structured UI action.
///
/// # Private members
/// - `worker`: Upstream-owned worker process and IPC state.
/// - `allowed_tools`: Tool identifiers authorized for this branch.
/// - `allowed_commands`: Slash command identifiers authorized by the host.
/// - `attestation`: Launcher-reported isolation recorded at startup.
pub struct AuthorizedPluginWorker {
    /// Retained upstream worker process and IPC state.
    worker: Arc<Mutex<IsolatedWorker>>,
    /// Tool identifiers authorized for this branch.
    allowed_tools: BTreeSet<String>,
    /// Slash command identifiers authorized by the host.
    allowed_commands: BTreeSet<String>,
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
            worker: Arc::new(Mutex::new(worker)),
            allowed_tools,
            allowed_commands: BTreeSet::new(),
            attestation,
        })
    }

    /// Starts a worker with separate tool and slash-command allowlists.
    ///
    /// - `spec`: Host-selected worker executable and environment.
    /// - `allowed_tools`: Tool names the host is willing to invoke.
    /// - `allowed_commands`: Slash command names the host is willing to invoke.
    /// - `allowed_operations`: Semantic operations the worker may request.
    /// - `dispatcher`: Host semantic operation route.
    /// - `launcher`: Host-injected OS sandbox launcher.
    /// - `sandbox`: Host-selected filesystem, network, and secret restrictions.
    /// - `protected_roots`: Canonical store, secret, and configuration paths the worker must not reach.
    ///
    /// # Returns
    /// A running isolated worker with its recorded sandbox attestation.
    ///
    /// # Errors
    /// Returns a sandbox, startup, or protocol error.
    #[allow(clippy::too_many_arguments)]
    pub fn start_with_commands(
        spec: &WorkerLaunchSpec,
        allowed_tools: BTreeSet<String>,
        allowed_commands: BTreeSet<String>,
        allowed_operations: BTreeSet<String>,
        dispatcher: Arc<dyn PluginSemanticDispatcher>,
        launcher: &dyn WorkerLauncher,
        sandbox: WorkerSandboxPolicy,
        protected_roots: &[PathBuf],
    ) -> Result<Self, WorkerError> {
        let mut worker = Self::start(
            spec,
            allowed_tools,
            allowed_operations,
            dispatcher,
            launcher,
            sandbox,
            protected_roots,
        )?;
        worker.allowed_commands = allowed_commands;
        Ok(worker)
    }

    /// Returns commands advertised by the worker and allowed by host policy.
    ///
    /// # Returns
    /// Authorized slash command descriptors in worker order.
    ///
    /// # Errors
    /// Returns a sandbox error when the worker process is not OS-isolated.
    pub fn advertised_commands(
        &self,
    ) -> Result<Vec<rpi_extensions::isolated::WorkerCommandSpec>, WorkerError> {
        self.require_sandbox()?;
        let worker = self
            .worker
            .lock()
            .map_err(|_| WorkerError::Closed("worker state is poisoned".to_owned()))?;
        Ok(worker
            .commands()
            .iter()
            .filter(|command| self.allowed_commands.contains(&command.name))
            .cloned()
            .collect())
    }

    /// Reports whether the retained upstream worker connection has closed.
    ///
    /// # Returns
    /// `true` when the worker cannot accept further commands or tools.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.worker.lock().map_or(true, |worker| worker.is_closed())
    }

    /// Executes one slash command authorized by the host.
    ///
    /// - `command`: Worker-advertised command name.
    /// - `arguments`: Raw arguments after the command name.
    ///
    /// # Returns
    /// The plugin's structured command response.
    ///
    /// # Errors
    /// Returns a sandbox, authorization, timeout, or worker failure.
    pub fn call_command(
        &self,
        command: &str,
        arguments: &str,
    ) -> Result<serde_json::Value, WorkerError> {
        self.call_command_action(command, arguments, None)
    }

    /// Executes one authorized command with a structured UI action.
    ///
    /// - `command`: Worker-advertised command name.
    /// - `arguments`: Raw arguments after the command name.
    /// - `action`: Optional response action from the command's prior UI result.
    ///
    /// # Returns
    /// The plugin's structured command response.
    ///
    /// # Errors
    /// Returns a sandbox, authorization, timeout, or worker failure.
    pub fn call_command_action(
        &self,
        command: &str,
        arguments: &str,
        action: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, WorkerError> {
        self.require_sandbox()?;
        if !self.allowed_commands.contains(command) {
            return Err(WorkerError::Unauthorized(command.to_owned()));
        }
        self.worker
            .lock()
            .map_err(|_| WorkerError::Closed("worker state is poisoned".to_owned()))?
            .call_command_action(command, arguments, action)
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
        let worker = self
            .worker
            .lock()
            .map_err(|_| WorkerError::Closed("worker state is poisoned".to_owned()))?;
        Ok(worker
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
    #[must_use]
    pub fn advertises(&self, tool: &str) -> bool {
        self.worker.lock().is_ok_and(|worker| {
            worker
                .tools()
                .iter()
                .any(|tool_spec| tool_spec.name == tool)
        })
    }

    /// Reports whether host policy allows one tool name.
    ///
    /// # Parameters
    /// - `tool`: Tool identifier selected by the provider.
    ///
    /// # Returns
    /// `true` when the host allowlist contains the exact name.
    #[must_use]
    pub fn allows(&self, tool: &str) -> bool {
        self.allowed_tools.contains(tool)
    }

    /// Returns the launcher-attested OS isolation for this worker.
    ///
    /// # Returns
    /// The guarantee set recorded when the worker started.
    #[must_use]
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
            .lock()
            .map_err(|_| WorkerError::Closed("worker state is poisoned".to_owned()))?
            .call_tool_cancellable(tool, arguments, cancellation)
    }

    /// Creates upstream provider adapters that match the host allowlist.
    ///
    /// # Parameters
    /// - `allowed_providers`: Provider ids the host explicitly authorizes.
    ///
    /// # Returns
    /// Adapters for advertised providers in worker order.
    ///
    /// # Errors
    /// Returns a sandbox error when the worker did not start under OS isolation.
    pub fn provider_adapters(
        &self,
        allowed_providers: &BTreeSet<String>,
    ) -> Result<Vec<Arc<dyn Provider>>, WorkerError> {
        self.require_sandbox()?;
        Ok(WorkerProvider::from_worker(Arc::clone(&self.worker))
            .into_iter()
            .filter(|provider| allowed_providers.contains(provider.id()))
            .collect())
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
