//! Validates isolated worker configuration before the host can use extensions.

#[cfg(feature = "isolated-plugin-worker")]
use super::extension_operations::DenyPluginOperations;
use scriberig_agent_runtime::host_configuration::HostConfigurationError;
#[cfg(feature = "isolated-plugin-worker")]
use scriberig_agent_runtime::plugin_worker::PluginWorkerConfiguration;
use scriberig_app::ProviderExtensionRuntime;
use scriberig_store_sqlite::SqliteStore;
#[cfg(feature = "isolated-plugin-worker")]
use std::collections::BTreeSet;
#[cfg(feature = "isolated-plugin-worker")]
use std::sync::Arc;

/// Validates plugin configuration before the native product starts execution.
///
/// Configured roots are resolved through upstream package APIs. A worker can
/// start only when this build includes isolated workers, a named capability is
/// configured, and the host's runtime sandbox probe succeeds. Branch execution
/// injects that launcher and still checks the immutable branch grant before
/// composing any worker tools.
///
/// # Returns
/// `Ok(())` when plugin configuration is absent or has a verified admission path.
///
/// # Errors
/// Returns a safe configuration error for invalid or unavailable extensions.
pub(crate) fn validate_isolated_plugin_configuration() -> Result<(), HostConfigurationError> {
    #[cfg(feature = "isolated-plugin-worker")]
    if let Some(configuration) = PluginWorkerConfiguration::from_environment().map_err(|error| {
        HostConfigurationError::Configuration(format!("invalid isolated plugin config: {error}"))
    })? {
        let has_tools = !configuration.allowed_tools.is_empty();
        let has_commands = !configuration.allowed_commands.is_empty();
        let has_providers = !configuration.allowed_providers.is_empty();
        if !has_tools && !has_commands && !has_providers {
            return Err(HostConfigurationError::Configuration(
                "configured plugins require an explicit tool, command, or provider allowlist"
                    .to_owned(),
            ));
        }
        if has_tools && configured_plugin_capability()?.is_none() {
            return Err(HostConfigurationError::Configuration(
                "configured plugin tools require SCRIBE_AGENT_PLUGIN_CAPABILITY".to_owned(),
            ));
        }
        if scriberig_agent_runtime::plugin_worker::sandboxed_launcher().is_none() {
            return Err(HostConfigurationError::Configuration(
                "configured plugins require a runtime-verified isolated worker launcher".to_owned(),
            ));
        }
    }
    #[cfg(not(feature = "isolated-plugin-worker"))]
    if [
        "SCRIBE_AGENT_PACKAGE_ROOTS",
        "SCRIBE_AGENT_EXTENSION_DIRS",
        "SCRIBE_AGENT_PLUGIN_FILES",
        "SCRIBE_AGENT_PLUGIN_PROVIDERS",
        "SCRIBE_AGENT_PLUGIN_COMMANDS",
    ]
    .into_iter()
    .any(path_list_is_nonempty)
    {
        return Err(HostConfigurationError::Configuration(
            "isolated plugin support is not enabled in this build".to_owned(),
        ));
    }
    Ok(())
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "the feature-enabled implementation reports fail-closed activation errors"
)]
/// Starts configured provider packages and publishes their models atomically.
///
/// # Parameters
/// - `runtime`: Shared provider and catalog generation runtime.
/// - `project_store`: Durable store whose data must remain inaccessible to the worker.
/// - `component_mailbox`: Upstream mailbox for host-mediated extension UI components.
///
/// # Returns
/// The admitted command worker when a command allowlist is configured.
///
/// # Errors
/// Returns a safe configuration error when package loading, sandbox admission,
/// provider validation, or atomic publication fails.
pub(crate) fn activate_provider_packages(
    runtime: &ProviderExtensionRuntime,
    project_store: &SqliteStore,
    component_mailbox: &rpi_extensions::ExtensionStatusMailbox,
) -> Result<
    Option<Arc<scriberig_agent_runtime::plugin_worker::AuthorizedPluginWorker>>,
    HostConfigurationError,
> {
    #[cfg(feature = "isolated-plugin-worker")]
    return activate_provider_packages_isolated(runtime, project_store, component_mailbox);
    #[cfg(not(feature = "isolated-plugin-worker"))]
    {
        let _ = (runtime, project_store, component_mailbox);
        Ok(None)
    }
}

#[cfg(feature = "isolated-plugin-worker")]
#[expect(
    clippy::too_many_lines,
    reason = "provider worker admission and generation publication form one fail-closed sequence"
)]
/// Loads allowlisted providers inside the verified isolated worker.
///
/// # Parameters
/// - `runtime`: Shared provider and catalog generation runtime.
/// - `project_store`: Durable store whose data must remain inaccessible to the worker.
/// - `component_mailbox`: Upstream mailbox for host-mediated extension UI components.
///
/// # Returns
/// `Ok(())` after the validated provider generation is published.
///
/// # Errors
/// Returns a configuration error if any configured provider cannot be admitted.
fn activate_provider_packages_isolated(
    runtime: &ProviderExtensionRuntime,
    project_store: &SqliteStore,
    component_mailbox: &rpi_extensions::ExtensionStatusMailbox,
) -> Result<
    Option<Arc<scriberig_agent_runtime::plugin_worker::AuthorizedPluginWorker>>,
    HostConfigurationError,
> {
    use rpi_extensions::GenerationCandidate;
    use scriberig_agent_runtime::plugin_worker::{
        PluginWorkerConfiguration, WorkerNetworkPolicy, WorkerSandboxPolicy, sandboxed_launcher,
    };
    use std::collections::BTreeSet;

    let Some(mut configuration) =
        PluginWorkerConfiguration::from_environment().map_err(|error| {
            HostConfigurationError::Configuration(format!(
                "invalid isolated plugin config: {error}"
            ))
        })?
    else {
        return Ok(None);
    };
    if configuration.allowed_providers.is_empty() && configuration.allowed_commands.is_empty() {
        return Ok(None);
    }
    let allowlist = configuration.allowed_providers.clone();
    let command_allowlist = configuration.allowed_commands.clone();
    let command_launch = configuration.launch.clone();
    configuration.allowed_tools.clear();
    configuration.allowed_commands.clear();
    configuration.allowed_operations = BTreeSet::from([
        "provider_http_request".to_owned(),
        "ui_component".to_owned(),
    ]);
    let launcher = sandboxed_launcher().ok_or_else(|| {
        HostConfigurationError::Configuration(
            "configured plugin providers require a runtime-verified isolated worker launcher"
                .to_owned(),
        )
    })?;
    let mut roots = crate::native_tui_agent::upstream_execution::plugin_read_roots(
        &configuration.launch.executable,
    );
    roots.extend(configuration.launch.environment.values().flat_map(|value| {
        std::env::split_paths(std::ffi::OsStr::new(value))
            .filter(|path| !path.as_os_str().is_empty())
            .map(|path| {
                if path.is_file() {
                    path.parent()
                        .map_or(path.clone(), std::path::Path::to_path_buf)
                } else {
                    path
                }
            })
            .collect::<Vec<_>>()
    }));
    let mut roots = crate::native_tui_agent::upstream_execution::canonical_roots(roots)
        .map_err(HostConfigurationError::Configuration)?;
    crate::native_tui_agent::upstream_execution::include_runtime_loader_aliases(&mut roots);
    let protected = crate::native_tui_agent::upstream_execution::protected_roots(project_store)
        .map_err(HostConfigurationError::Configuration)?;
    let mut sandbox = WorkerSandboxPolicy::sandboxed(roots, Vec::new());
    sandbox.network = WorkerNetworkPolicy::Deny;
    let command_worker = if command_allowlist.is_empty() {
        None
    } else {
        let command_configuration = PluginWorkerConfiguration {
            allowed_tools: BTreeSet::new(),
            allowed_commands: command_allowlist,
            allowed_operations: BTreeSet::from(["ui_component".to_owned()]),
            allowed_providers: BTreeSet::new(),
            launch: command_launch,
        };
        let mut command_sandbox = sandbox.clone();
        command_sandbox.network = WorkerNetworkPolicy::Allow;
        Some(Arc::new(
            command_configuration
                .start(
                    Arc::new(DenyPluginOperations::new(component_mailbox.clone())),
                    launcher,
                    command_sandbox,
                    &protected,
                )
                .map_err(|error| {
                    HostConfigurationError::Configuration(format!(
                        "isolated plugin command startup failed: {error}"
                    ))
                })?,
        ))
    };
    if allowlist.is_empty() {
        return Ok(command_worker);
    }
    let provider_operations = Arc::new(DenyPluginOperations::new(component_mailbox.clone()));
    let worker = Arc::new(
        configuration
            .start(provider_operations.clone(), launcher, sandbox, &protected)
            .map_err(|error| {
                HostConfigurationError::Configuration(format!(
                    "isolated plugin provider startup failed: {error}"
                ))
            })?,
    );
    let providers = worker.provider_adapters(&allowlist).map_err(|error| {
        HostConfigurationError::Configuration(format!(
            "isolated plugin provider discovery failed: {error}"
        ))
    })?;
    let observed = providers
        .iter()
        .map(|provider| provider.id().to_owned())
        .collect::<BTreeSet<_>>();
    if observed != allowlist {
        return Err(HostConfigurationError::Configuration(
            "configured plugin provider allowlist does not match worker registrations".to_owned(),
        ));
    }
    let models = validated_provider_models(runtime, &providers)?;
    provider_operations.set_provider_models(
        providers
            .iter()
            .flat_map(|provider| provider.models().iter().cloned())
            .collect(),
    );
    let before = runtime.extension_catalog_snapshot();
    let candidate = GenerationCandidate::from_generation(&before.extension)
        .retain(Arc::clone(&worker) as Arc<dyn std::any::Any + Send + Sync>);
    runtime
        .refresh_provider_extension_generation(|| Ok::<_, String>((candidate, models, providers)))
        .map_err(|error| {
            HostConfigurationError::Configuration(format!(
                "plugin provider generation publication failed: {error:?}"
            ))
        })?;
    Ok(command_worker)
}

/// Validates provider identities and appends their models to the current catalog.
///
/// # Parameters
/// - `runtime`: Shared provider and catalog generation runtime.
/// - `providers`: Worker registrations admitted by the explicit provider allowlist.
///
/// # Returns
/// Existing catalog models plus each validated provider model.
///
/// # Errors
/// Returns a configuration error for invalid endpoints or conflicting model IDs.
#[cfg(feature = "isolated-plugin-worker")]
fn validated_provider_models(
    runtime: &ProviderExtensionRuntime,
    providers: &[Arc<dyn rpi_ai::Provider>],
) -> Result<Vec<rpi_ai::Model>, HostConfigurationError> {
    let (snapshot, active_provider_ids) = runtime.provider_generation_snapshot();
    let active_provider_ids = active_provider_ids.into_iter().collect::<BTreeSet<_>>();
    let mut models = snapshot
        .catalog
        .models
        .into_iter()
        .filter(|model| !active_provider_ids.contains(&model.provider))
        .collect::<Vec<_>>();
    for provider in providers {
        for model in provider.models() {
            scriberig_secrets::validate_public_endpoint(&model.base_url).map_err(|_| {
                HostConfigurationError::Configuration(
                    "plugin provider advertised an invalid public endpoint".to_owned(),
                )
            })?;
            if model.provider.as_str() != provider.id()
                || models
                    .iter()
                    .any(|existing| existing.provider == model.provider && existing.id == model.id)
            {
                return Err(HostConfigurationError::Configuration(
                    "plugin provider advertised a conflicting model identity".to_owned(),
                ));
            }
            models.push(model.clone());
        }
    }
    Ok(models)
}

/// Adds the configured plugin capability to a host-authored context.
///
/// The capability is granted only by the local host configuration and only in
/// builds that can run isolated workers. Callers still pass the resulting
/// context through normal branch delegation filters.
///
/// # Parameters
/// - `context`: Host-owned context whose explicit grants may be extended.
///
/// Grants a configured plugin capability to a host-authored context.
///
/// # Parameters
/// - `context`: Host-owned context whose explicit grants may be extended.
pub(crate) fn grant_configured_plugin_capability(context: &mut scriberig_app::ApplicationContext) {
    #[cfg(feature = "isolated-plugin-worker")]
    if let Ok(Some(capability)) = configured_plugin_capability() {
        if !context.capabilities.allows(&capability) {
            context.capabilities.granted.push(capability);
        }
    }
    #[cfg(not(feature = "isolated-plugin-worker"))]
    let _ = context;
}

/// Parses the locally configured plugin capability.
///
/// # Returns
/// The configured capability, or `None` when no name is set.
///
/// # Errors
/// Returns a configuration error when the name is invalid.
#[cfg(feature = "isolated-plugin-worker")]
fn configured_plugin_capability()
-> Result<Option<scriberig_app::CapabilityId>, HostConfigurationError> {
    std::env::var("SCRIBE_AGENT_PLUGIN_CAPABILITY")
        .ok()
        .map(scriberig_app::CapabilityId::new)
        .transpose()
        .map_err(|_| {
            HostConfigurationError::Configuration(
                "configured plugin capability name is invalid".to_owned(),
            )
        })
}

/// Checks whether an environment path list contains a non-empty path.
///
/// # Parameters
/// - `name`: Environment variable containing configured native-code paths.
///
/// # Returns
/// `true` when the process requested any extension path.
#[cfg(not(feature = "isolated-plugin-worker"))]
fn path_list_is_nonempty(name: &str) -> bool {
    std::env::var_os(name)
        .is_some_and(|value| std::env::split_paths(&value).any(|path| !path.as_os_str().is_empty()))
}
