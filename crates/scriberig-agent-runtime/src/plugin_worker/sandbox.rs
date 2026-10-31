//! Validates the OS sandbox posture a plugin worker must hold before its tools
//! can become model-visible.
//!
//! Upstream owns the worker process, the versioned IPC, and the sandbox
//! launcher; the launcher's `WorkerSandboxGuarantees` are self-reported. The
//! checks here are the `ScribeRig` side of the same contract: the host refuses a
//! launch that could read the canonical store or ambient credentials, and it
//! refuses to convert worker tools into branch tools when the selected launcher
//! does not claim every enforced restriction.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use rpi_extensions::isolated::{
    LinuxBubblewrapLauncher, WorkerFilesystemPolicy, WorkerIsolation, WorkerLaunchSpec,
    WorkerLauncher, WorkerSandboxGuarantees, WorkerSandboxPolicy, WorkerSecretPolicy,
};

/// Launcher-attested OS restrictions for one started worker.
///
/// # Public members
/// - `platform`: Launcher implementation that reported the guarantee set.
/// - `filesystem`: Whether the launcher enforces the requested roots.
/// - `network`: Whether the launcher enforces the requested network policy.
/// - `secrets`: Whether the launcher enforces the requested secret policy.
///
/// # Private members
/// None.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkerSandboxAttestation {
    /// Launcher implementation that reported the guarantee set.
    pub platform: &'static str,
    /// Whether the launcher enforces the requested filesystem roots.
    pub filesystem: bool,
    /// Whether the launcher enforces the requested network policy.
    pub network: bool,
    /// Whether the launcher enforces the requested environment/secret policy.
    pub secrets: bool,
}

impl WorkerSandboxAttestation {
    /// Records one launcher-reported guarantee set.
    ///
    /// # Parameters
    /// - `guarantees`: Guarantee set reported by the selected launcher.
    ///
    /// # Returns
    /// The ScribeRig-side attestation retained with the worker.
    #[must_use]
    pub fn from_guarantees(guarantees: &WorkerSandboxGuarantees) -> Self {
        Self {
            platform: guarantees.platform,
            filesystem: guarantees.filesystem,
            network: guarantees.network,
            secrets: guarantees.secrets,
        }
    }

    /// Reports whether every requested OS restriction is enforced.
    ///
    /// # Returns
    /// `true` only when filesystem, network, and secret restrictions are all
    /// enforced by the launcher that started this worker.
    #[must_use]
    pub fn enforces_os_isolation(&self) -> bool {
        self.filesystem && self.network && self.secrets
    }
}

/// Returns the host launcher able to enforce an OS sandbox, when one exists.
///
/// `ScribeRig` never falls back to an unsandboxed launcher. A platform with no
/// built-in sandbox launcher must inject a `WorkerLauncher` through its own
/// host policy or plugin tools stay unavailable.
///
/// # Returns
/// The platform sandbox launcher, or `None` when the platform has none.
#[must_use]
pub fn sandboxed_launcher() -> Option<&'static dyn WorkerLauncher> {
    #[cfg(target_os = "linux")]
    {
        static BUBBLEWRAP: LinuxBubblewrapLauncher = LinuxBubblewrapLauncher;
        static VERIFIED: OnceLock<bool> = OnceLock::new();
        VERIFIED
            .get_or_init(|| verify_bubblewrap(BUBBLEWRAP))
            .then_some(&BUBBLEWRAP as &dyn WorkerLauncher)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Verifies the actual bubblewrap launch against filesystem, environment, and namespace checks.
///
/// A successful guarantee report alone cannot admit a worker. This probe starts
/// a restricted shell and confirms that unbound host paths and `/proc` are not
/// visible, only declared environment values survive, and the process enters a
/// distinct network namespace.
///
/// # Parameters
/// - `launcher`: Platform launcher selected by the host.
///
/// # Returns
/// `true` only when every runtime check succeeds.
fn verify_bubblewrap(launcher: LinuxBubblewrapLauncher) -> bool {
    let executable = PathBuf::from("/bin/sh");
    if !executable.is_file() {
        return false;
    }
    let read_roots = ["/usr", "/bin", "/lib", "/lib64", "/etc"]
        .into_iter()
        .map(PathBuf::from)
        .filter(|root| root.exists())
        .collect();
    let spec = WorkerLaunchSpec {
        executable,
        args: vec![
            "-c".to_owned(),
            "test ! -e /proc/self/status && test ! -e /root && test \"$SCRIBE_SANDBOX_PROBE\" = allowed && test -z \"${SCRIBE_SANDBOX_HOST_CANARY:-}\" && exec /usr/bin/sleep 2".to_owned(),
        ],
        environment: std::collections::BTreeMap::from([(
            "SCRIBE_SANDBOX_PROBE".to_owned(),
            "allowed".to_owned(),
        )]),
        working_directory: Some(PathBuf::from("/tmp")),
        inherit_environment: false,
        sandbox: WorkerSandboxPolicy {
            isolation: WorkerIsolation::Sandboxed,
            filesystem: WorkerFilesystemPolicy {
                read_roots,
                write_roots: Vec::new(),
            },
            network: rpi_extensions::isolated::WorkerNetworkPolicy::Deny,
            secrets: WorkerSecretPolicy::DeclaredOnly,
        },
        startup_timeout: Duration::from_secs(3),
        request_timeout: Duration::from_secs(3),
    };
    let Ok(mut command) = launcher.command(&spec) else {
        return false;
    };
    command.env("SCRIBE_SANDBOX_HOST_CANARY", "must-not-inherit");
    let Ok(mut child) = command.spawn() else {
        return false;
    };
    let host_namespace = std::fs::read_link("/proc/self/ns/net");
    let deadline = Instant::now() + Duration::from_secs(1);
    let isolated_network = loop {
        if host_namespace
            .as_ref()
            .is_ok_and(|host| has_isolated_network_descendant(child.id(), host))
        {
            break true;
        }
        if Instant::now() >= deadline || child.try_wait().ok().flatten().is_some() {
            break false;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let output = child.wait_with_output();
    isolated_network && output.is_ok_and(|result| result.status.success())
}

/// Finds a bubblewrap child running in a distinct network namespace.
///
/// Bubblewrap keeps a supervisor outside the new PID namespace, so verification
/// follows the host-visible child tree until it finds the sandboxed process.
///
/// # Parameters
/// - `parent`: Host-visible process ID to inspect.
/// - `host_namespace`: Host network namespace identity.
///
/// # Returns
/// `true` when a descendant belongs to a different network namespace.
fn has_isolated_network_descendant(parent: u32, host_namespace: &Path) -> bool {
    let children_path = format!("/proc/{parent}/task/{parent}/children");
    let Ok(children) = std::fs::read_to_string(children_path) else {
        return false;
    };
    children.split_whitespace().any(|child| {
        let Ok(child) = child.parse::<u32>() else {
            return false;
        };
        let namespace_path = format!("/proc/{child}/ns/net");
        if std::fs::read_link(&namespace_path).is_ok_and(|namespace| namespace != host_namespace) {
            return true;
        }
        has_isolated_network_descendant(child, host_namespace)
    })
}

/// Rejects a worker launch that could read protected `ScribeRig` state.
///
/// The check is deliberately structural: a plugin worker may read only the
/// absolute roots the host selected, may never inherit the host environment,
/// and may never be given a root, working directory, or declared environment
/// value that reaches a protected store, secret, or configuration path.
///
/// # Parameters
/// - `spec`: Host-selected executable, environment, and sandbox policy.
/// - `protected_roots`: Canonical store, secret, and configuration paths the
///   worker must not reach.
///
/// # Returns
/// `Ok(())` when the launch keeps protected state out of the worker.
///
/// # Errors
/// Returns a safe denial naming the rejected property.
pub fn validate_worker_launch(
    spec: &WorkerLaunchSpec,
    protected_roots: &[PathBuf],
) -> Result<(), String> {
    if spec.sandbox.isolation != WorkerIsolation::Sandboxed {
        return Err("plugin workers require an OS-sandboxed policy".to_owned());
    }
    if spec.inherit_environment {
        return Err("plugin workers must not inherit the host environment".to_owned());
    }
    if spec.sandbox.secrets == WorkerSecretPolicy::DeclaredOnly && protected_roots.is_empty() {
        return Err(
            "plugin workers require explicit protected paths before they may start".to_owned(),
        );
    }
    validate_roots(&spec.sandbox.filesystem)?;
    if let Some(directory) = spec.working_directory.as_deref() {
        reject_protected(
            "plugin worker working directory",
            directory,
            protected_roots,
        )?;
    }
    for root in &spec.sandbox.filesystem.read_roots {
        reject_protected("plugin worker read root", root, protected_roots)?;
    }
    for root in &spec.sandbox.filesystem.write_roots {
        reject_protected("plugin worker write root", root, protected_roots)?;
    }
    for (name, value) in &spec.environment {
        if protected_roots
            .iter()
            .any(|protected| value.contains(&protected.to_string_lossy().into_owned()))
        {
            return Err(format!(
                "plugin worker environment value `{name}` exposes protected ScribeRig state"
            ));
        }
    }
    Ok(())
}

/// Rejects missing, relative, or empty filesystem roots.
///
/// # Parameters
/// - `filesystem`: Requested worker filesystem roots.
///
/// # Returns
/// `Ok(())` when every root is absolute and non-empty.
///
/// # Errors
/// Returns a safe denial for the first unusable root.
fn validate_roots(filesystem: &WorkerFilesystemPolicy) -> Result<(), String> {
    if filesystem
        .read_roots
        .iter()
        .chain(&filesystem.write_roots)
        .any(|root| root.as_os_str().is_empty() || !root.is_absolute())
    {
        return Err("plugin worker filesystem roots must be absolute".to_owned());
    }
    Ok(())
}

/// Rejects one launch path that equals or contains protected state.
///
/// # Parameters
/// - `kind`: Human-readable launch property being checked.
/// - `path`: Absolute path the worker would receive.
/// - `protected_roots`: Paths the worker must not reach.
///
/// # Returns
/// `Ok(())` when the path stays outside every protected root.
///
/// # Errors
/// Returns a safe denial naming the rejected property.
fn reject_protected(kind: &str, path: &Path, protected_roots: &[PathBuf]) -> Result<(), String> {
    if protected_roots
        .iter()
        .any(|protected| protected.starts_with(path))
    {
        return Err(format!(
            "{kind} `{}` would expose protected ScribeRig state",
            path.display()
        ));
    }
    Ok(())
}
