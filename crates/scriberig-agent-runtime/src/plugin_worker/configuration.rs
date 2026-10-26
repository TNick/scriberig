//! Reads host-selected package roots and configures isolated worker launches.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use rpi_extensions::isolated::{WorkerLaunchSpec, WorkerLauncher, WorkerSandboxPolicy};

use super::{AuthorizedPluginWorker, PluginSemanticDispatcher, packages};

/// Host configuration for an isolated plugin worker.
///
/// # Public members
/// - `allowed_tools`: Worker tool IDs the host may invoke.
/// - `allowed_commands`: Slash command IDs the host may invoke.
/// - `allowed_operations`: Semantic operations the worker may request.
/// - `allowed_providers`: Provider ids the host may select for model turns.
/// - `launch`: Executable and bounded environment for the worker process.
///
/// # Private members
/// None.
pub struct PluginWorkerConfiguration {
    /// Allowed worker tool identifiers.
    pub allowed_tools: BTreeSet<String>,
    /// Allowed worker slash command identifiers.
    pub allowed_commands: BTreeSet<String>,
    /// Allowed worker semantic operation identifiers.
    pub allowed_operations: BTreeSet<String>,
    /// Provider identifiers the host may select from worker registrations.
    pub allowed_providers: BTreeSet<String>,
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
        let mut directories = path_list("SCRIBE_AGENT_EXTENSION_DIRS");
        let mut project_roots = path_list("SCRIBE_AGENT_PACKAGE_ROOTS");
        if let Ok(project_root) = std::env::current_dir()
            && packages::has_project_package_registry(&project_root)
            && !project_roots.contains(&project_root)
        {
            project_roots.push(project_root);
        }
        directories.extend(packages::package_extension_dirs(&project_roots)?);
        let directories = packages::canonical_directories(directories)?;
        let files = packages::canonical_files(path_list("SCRIBE_AGENT_PLUGIN_FILES"))?;
        if directories.is_empty() && files.is_empty() {
            if !name_set("SCRIBE_AGENT_PLUGIN_TOOLS").is_empty()
                || !name_set("SCRIBE_AGENT_PLUGIN_COMMANDS").is_empty()
                || !name_set("SCRIBE_AGENT_PLUGIN_PROVIDERS").is_empty()
            {
                return Err("plugin allowlists require a configured package root".to_owned());
            }
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
        let mut allowed_operations = name_set("SCRIBE_AGENT_PLUGIN_CAPABILITIES");
        allowed_operations.insert("ui_component".to_owned());
        Ok(Some(Self {
            allowed_tools: name_set("SCRIBE_AGENT_PLUGIN_TOOLS"),
            allowed_commands: name_set("SCRIBE_AGENT_PLUGIN_COMMANDS"),
            allowed_operations,
            allowed_providers: name_set("SCRIBE_AGENT_PLUGIN_PROVIDERS"),
            launch,
        }))
    }

    /// Starts the configured worker.
    ///
    /// # Parameters
    /// - `self`: Host worker configuration.
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
    ) -> Result<AuthorizedPluginWorker, rpi_extensions::isolated::WorkerError> {
        let Self {
            allowed_tools,
            allowed_commands,
            allowed_operations,
            allowed_providers: _,
            launch,
        } = self;
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
