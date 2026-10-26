//! Composes one explicitly admitted plugin worker into one branch tool set.
//!
//! This is the single `ScribeRig` entry point that may start a plugin worker and
//! turn its tools into branch tools. It reads nothing from the environment: the
//! host injects the sandbox launcher it selected plus the tool allowlist and
//! launch policy it is willing to invoke, so a build or platform with no
//! verified launcher cannot reach a worker through configuration alone. The
//! entry point fails closed before any process starts when no launcher is
//! injected, and it delegates every remaining check to the existing seam:
//! `AuthorizedPluginWorker::start` refuses a launcher that does not report
//! enforcing the requested filesystem, network, and secret restrictions and a
//! launch that could reach a protected path, and `PluginToolBridge::compose`
//! adapts only a tool that is advertised, allowlisted, and granted to this
//! branch.
//!
//! The started worker is kept alive by the composed adapters; the upstream
//! `IsolatedWorker` shuts its supervised process tree down once the last adapter
//! is dropped, so a composition that yields no branch-visible tool leaves no
//! running worker behind.

use std::path::PathBuf;
use std::sync::Arc;

use rpi_extensions::isolated::{WorkerLauncher, WorkerSandboxPolicy};

use crate::plugin_worker::{PluginSemanticDispatcher, PluginWorkerConfiguration};

use super::bridge::{
    ComposedPluginTools, PluginCompositionError, PluginToolBridge, PluginToolRequest,
};

/// Host-supplied inputs that may admit one plugin worker to one branch.
///
/// The launcher guarantee set is launcher self-report. The host stays
/// responsible for selecting a launcher it verified; this admission refuses any
/// launcher that reports no OS enforcement, which is a necessary condition and
/// not proof that the sandbox holds.
///
/// # Public members
/// - `launcher`: Host-injected OS sandbox launcher; `None` fails closed.
/// - `configuration`: Host allowlist plus the worker executable and declared
///   environment.
/// - `dispatcher`: Host route for worker semantic operation requests.
/// - `sandbox`: Host-selected filesystem, network, and secret restrictions.
/// - `protected_roots`: Canonical store, secret, and configuration paths the
///   worker must not reach.
/// - `capability`: Plugin capability a branch must hold to see these tools.
///
/// # Private members
/// None.
pub struct PluginWorkerAdmission<'launcher> {
    /// Launcher the host selected and verified, or `None` when it has none.
    pub launcher: Option<&'launcher dyn WorkerLauncher>,
    /// Host tool allowlist and bounded worker launch specification.
    pub configuration: PluginWorkerConfiguration,
    /// Host semantic operation route for worker host calls.
    pub dispatcher: Arc<dyn PluginSemanticDispatcher>,
    /// Host-selected filesystem, network, and secret restrictions.
    pub sandbox: WorkerSandboxPolicy,
    /// Canonical store, secret, and configuration paths to keep unreachable.
    pub protected_roots: Vec<PathBuf>,
    /// Plugin capability recorded on every composed tool.
    pub capability: String,
}

/// Starts one admitted worker and merges its authorized tools into a branch.
///
/// A supplied-but-unverified launcher, an unisolated policy, and a launch that
/// could reach a protected root are all refused by `AuthorizedPluginWorker::start`
/// before the worker process is spawned. After the worker starts, a tool the
/// host does not allowlist or the branch capability snapshot does not grant is
/// never adapted, so an unactivated plugin capability stays out of
/// provider-visible context.
///
/// # Parameters
/// - `admission`: Host-injected launcher, tool allowlist, and launch policy.
/// - `request`: Trusted catalog, capability snapshot, branch identity, activity
///   sequence, and durable evidence route for this branch.
///
/// # Returns
/// The merged branch catalog plus one adapter per branch-visible plugin tool.
///
/// # Errors
/// Returns `PluginCompositionError` when no launcher was injected, the injected
/// launcher or launch policy is refused, the worker could not start, or the
/// merged catalog or an upstream adapter refuses a tool specification.
pub fn compose_branch_plugin_tools(
    admission: PluginWorkerAdmission<'_>,
    request: PluginToolRequest,
) -> Result<ComposedPluginTools, PluginCompositionError> {
    let PluginWorkerAdmission {
        launcher,
        configuration,
        dispatcher,
        sandbox,
        protected_roots,
        capability,
    } = admission;
    let Some(launcher) = launcher else {
        return Err(PluginCompositionError::SandboxUnavailable(
            "the host injected no verified plugin worker launcher".to_owned(),
        ));
    };
    let worker = configuration
        .start(dispatcher, launcher, sandbox, &protected_roots)
        .map_err(|error| PluginCompositionError::WorkerStart(error.to_string()))?;
    let bridge = PluginToolBridge::new(Arc::new(worker), &capability)?;
    bridge.compose(request)
}
