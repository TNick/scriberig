//! Composes the native TUI with the shared provider and agent orchestration runtime.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;

use rpi_ai::{Model, Provider};
use scriberig_agent_runtime::host_configuration::HostConfigurationError;
use scriberig_app::{
    AgentSelection, ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use crate::native_scenario::NativeScenario;

#[cfg(test)]
#[path = "native_tui_agent/continuation_scenario.rs"]
pub(crate) mod continuation_scenario;

#[path = "native_tui_agent/workspace.rs"]
mod workspace;
use workspace::workspace_from_result;
#[path = "native_tui_agent/child_selection.rs"]
mod child_selection;
#[path = "native_tui_agent/model_properties.rs"]
mod model_properties;
#[path = "native_tui_agent/model_selection.rs"]
mod model_selection;
#[path = "native_tui_agent/plan_worker.rs"]
mod plan_worker;
#[path = "native_tui_agent/provider_config.rs"]
mod provider_config;
#[path = "native_tui_agent/provider_readiness.rs"]
mod provider_readiness;
#[path = "native_tui_agent/runner.rs"]
mod runner;
#[path = "native_tui_agent/upstream_execution/mod.rs"]
pub(crate) mod upstream_execution;
use model_selection::selected_runtime_model;
use plan_worker::NativePlanWorkerExecutor;
pub(crate) use provider_config::extensions::{
    grant_configured_plugin_capability, validate_isolated_plugin_configuration,
};

/// Activates provider packages for a headless one-shot operation.
///
/// The private mailbox is retained by the worker for the duration of the
/// operation and discarded when its worker generation is released.
///
/// # Parameters
/// - `runtime`: Shared provider and catalog generation runtime.
/// - `project_store`: Durable store the worker must not access.
///
/// # Returns
/// The admitted provider worker, when configured.
///
/// # Errors
/// Returns a safe configuration error when package admission fails.
pub(crate) fn activate_provider_packages(
    runtime: &scriberig_app::ProviderExtensionRuntime,
    project_store: &SqliteStore,
) -> Result<
    Option<Arc<scriberig_agent_runtime::plugin_worker::AuthorizedPluginWorker>>,
    HostConfigurationError,
> {
    let mailbox = rpi_extensions::ExtensionStatusMailbox::new();
    provider_config::extensions::activate_provider_packages(runtime, project_store, &mailbox)
}

/// Builds the upstream adapter for the configured one-shot Echo fallback.
///
/// # Parameters
/// - `model`: Exact faux model descriptor used by the one-shot CLI invocation.
///
/// # Returns
/// A provider that executes deterministic Echo responses through rpi-agent.
pub(crate) fn echo_provider(model: Model) -> Arc<dyn Provider> {
    NativeScenario::Echo.into_provider(model)
}
#[cfg(test)]
#[path = "native_tui_agent/test_execution.rs"]
mod test_execution;
#[cfg(test)]
pub(crate) use test_execution::execute_agent;
#[cfg(test)]
#[path = "native_tui_agent_tests.rs"]
mod tests;

/// Runs provider-backed agent commands submitted through the native TUI.
pub(super) struct NativeTuiAgentRunner {
    /// Configured adapters keyed by the model library's safe connection IDs.
    providers: std::collections::BTreeMap<String, NativeScenario>,
    /// Model identifier selected by the local runtime configuration.
    model: String,
    /// Host-authorized credentials for the shared provider configuration.
    upstream_credentials: Option<crate::agent_config::AuthorizedCredentialProvider>,
    /// Whether the selected process configuration supports upstream providers.
    upstream_enabled: bool,
    /// Upstream catalog retained as the model selection authority.
    model_catalog: rpi_model_runtime::ModelCatalog,
    /// Shared catalog and extension publication generation used by app surfaces.
    provider_extension_runtime: scriberig_app::ProviderExtensionRuntime,
    /// Explicitly allowlisted plugin command worker shared with TUI surfaces.
    plugin_command_worker:
        Mutex<Option<Arc<scriberig_agent_runtime::plugin_worker::AuthorizedPluginWorker>>>,
    /// Session-scoped mailbox for isolated plugin terminal components.
    extension_component_mailbox: rpi_extensions::ExtensionStatusMailbox,
    /// Shared upstream provider lifecycle event source for active invocations.
    provider_lifecycle: provider_readiness::ProviderLifecycleState,
    /// Sanitized host model configuration updated through upstream validators.
    model_config: Mutex<(rpi_model_runtime::ModelsConfig, Option<std::path::PathBuf>)>,
    /// Shared project store used by application-backed agent tools.
    project_store: Arc<SqliteStore>,
    /// In-process guard against starting two coordinator loops for one plan.
    active_coordinators: Mutex<HashSet<EntityId>>,
    /// Root-only ephemeral continuation handles keyed by canonical branch.
    continuations: scriberig_agent_runtime::continuation::EphemeralContinuationOwner,
}

impl NativeTuiAgentRunner {
    /// Builds the TUI agent runtime from trusted process configuration.
    ///
    /// - `project_store`: Shared durable project and semantic store.
    /// - `super`: Parent module containing the native runtime composition.
    ///
    /// A runner configured from shared application settings and optional TUI overrides.
    ///
    /// Returns a structured provider configuration failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub(super) fn from_environment(
        project_store: Arc<SqliteStore>,
    ) -> Result<Self, HostConfigurationError> {
        let provider_kind = std::env::var("SCRIBE_AGENT_PROVIDER")
            .unwrap_or_else(|_| "openai_compatible".to_owned());
        let endpoint_override = std::env::var("SCRIBE_AGENT_BASE_URL").ok();
        let shared_config = if provider_kind == "openai_compatible" && endpoint_override.is_none() {
            crate::agent_config::resolve_provider_config()
                .map_err(|error| HostConfigurationError::Configuration(error.message))?
        } else {
            None
        };
        let upstream_credentials = match shared_config.as_ref() {
            Some((_, credentials, _)) => credentials.clone(),
            None if provider_kind == "openai_compatible" && endpoint_override.is_some() => {
                legacy_endpoint_credentials()?
            }
            None => None,
        };
        let upstream_enabled = provider_kind == "openai_compatible";
        let configured_model = shared_config
            .as_ref()
            .map_or("default", |(model, _, _)| model.id.as_str());
        let model =
            std::env::var("SCRIBE_AGENT_MODEL").unwrap_or_else(|_| configured_model.to_owned());
        let endpoint_override = endpoint_override.or_else(|| {
            shared_config
                .as_ref()
                .map(|(model, _, _)| model.base_url.clone())
        });
        let catalog =
            provider_config::catalog::from_environment(&model, endpoint_override.as_deref())?;
        let provider_lifecycle = provider_readiness::ProviderLifecycleState::new();
        let mut providers = std::collections::BTreeMap::new();
        if !upstream_enabled
            && let Some(provider) = provider_config::from_environment(&provider_kind)?
        {
            providers.insert("local".to_owned(), provider);
        }
        provider_config::extensions::validate_isolated_plugin_configuration()?;
        let provider_extension_runtime =
            scriberig_app::ProviderExtensionRuntime::with_model_catalog(
                catalog.model_catalog.clone(),
            );
        let extension_component_mailbox = rpi_extensions::ExtensionStatusMailbox::new();
        let plugin_command_worker = provider_config::extensions::activate_provider_packages(
            &provider_extension_runtime,
            &project_store,
            &extension_component_mailbox,
        )?;
        Ok(Self {
            providers,
            model,
            upstream_credentials,
            upstream_enabled,
            model_catalog: provider_extension_runtime.model_catalog().clone(),
            provider_extension_runtime,
            plugin_command_worker: Mutex::new(plugin_command_worker),
            extension_component_mailbox,
            provider_lifecycle,
            model_config: Mutex::new((catalog.config, catalog.config_path)),
            project_store,
            active_coordinators: Mutex::new(HashSet::new()),
            continuations: scriberig_agent_runtime::continuation::EphemeralContinuationOwner::new(),
        })
    }

    /// Returns the shared provider and extension publication runtime.
    ///
    /// # Returns
    /// A cloneable runtime handle sharing catalog and generation state.
    pub(super) fn provider_extension_runtime(&self) -> scriberig_app::ProviderExtensionRuntime {
        self.provider_extension_runtime.clone()
    }

    /// Returns a lifecycle event sink bound to the current provider catalog.
    ///
    /// # Returns
    /// An invocation context that reports provider results against the exact
    /// upstream model catalog revision observed at creation time.
    pub(super) fn provider_lifecycle_context(
        &self,
    ) -> scriberig_agent_runtime::upstream_invocation::ProviderLifecycleContext {
        let snapshot = self.model_catalog.snapshot();
        if let Ok(locations) = scriberig_secrets::registry::RegistryLocations::from_process()
            && let Ok(registry) = locations.load()
        {
            self.provider_lifecycle.observe_registry_revision(
                registry.credential_revision,
                snapshot.models.iter().map(|model| model.provider.clone()),
            );
        }
        self.provider_lifecycle.context(snapshot.revision)
    }
}

/// Builds the protected host credential source for a legacy endpoint override.
///
/// # Returns
/// An authorized environment reference or an unauthenticated source when no
/// credential reference was configured.
///
/// # Errors
/// Returns a safe configuration error when the reference is invalid or the
/// host credential policy cannot be loaded.
fn legacy_endpoint_credentials()
-> Result<Option<crate::agent_config::AuthorizedCredentialProvider>, HostConfigurationError> {
    let Some(name) = std::env::var("SCRIBE_AGENT_API_KEY_ENV").ok() else {
        return Ok(None);
    };
    let reference = scriberig_secrets::SecretRef::Environment { name };
    reference.validate().map_err(|_| {
        HostConfigurationError::Configuration("invalid credential reference".to_owned())
    })?;
    crate::agent_config::authorized_credential_provider_for_check(Some(&reference)).map_err(|_| {
        HostConfigurationError::Configuration("credential policy is unavailable".to_owned())
    })
}

/// Resolves the latest durable project scope produced in a session.
///
/// - `service`: Shared application boundary owning durable session state.
/// - `context`: Trusted authorization context for session reads.
/// - `session`: Session receiving the next user instruction.
///
/// Exact project, root, selected entity, and snapshot when the session has an
/// onboarding result.
///
/// Returns session read or malformed durable-handle failures.
fn previous_workspace_selection<B: ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
) -> Result<Option<AgentSelection>, ApplicationError> {
    let saved = match SessionBackend::read_session(service, context, &SessionRequest { session }) {
        Ok(saved) => saved,
        Err(ApplicationError::NotFound { .. }) => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some(handles) = saved.branches.iter().rev().find_map(|branch| {
        let handles = &branch.result.as_ref()?.result_handles;
        workspace_handle(handles, "project")
            .is_some()
            .then_some(handles)
            .filter(|handles| {
                workspace_handle(handles, "snapshot").is_some()
                    && workspace_handle(handles, "root").is_some()
            })
    }) else {
        return Ok(None);
    };
    let project = workspace_handle(handles, "project").expect("complete workspace handle set");
    let snapshot = workspace_handle(handles, "snapshot").expect("complete workspace handle set");
    let root = workspace_handle(handles, "root").expect("complete workspace handle set");
    let entity = handle_value(handles, "entity").unwrap_or(root);
    Ok(Some(AgentSelection {
        project: Some(parse_workspace_handle("project", project)?),
        root: parse_workspace_handle("root", root)?,
        entity: parse_workspace_handle("entity", entity)?,
        snapshot: parse_workspace_handle("snapshot", snapshot)?,
    }))
}

/// Finds one labeled workspace handle or a compatible legacy typed identity.
///
/// # Parameters
/// - `handles`: Durable result handles recorded by an earlier agent turn.
/// - `kind`: Label used by the current tagged handle format.
///
/// # Returns
/// The exact labeled value, or the first legacy value with the requested ID type.
fn workspace_handle<'a>(handles: &'a [String], kind: &str) -> Option<&'a str> {
    handle_value(handles, kind).or_else(|| {
        handles.iter().map(String::as_str).find(|value| match kind {
            "project" => value.parse::<scriberig_core::ProjectId>().is_ok(),
            "snapshot" => value.parse::<scriberig_core::SnapshotId>().is_ok(),
            "root" | "entity" => value.parse::<scriberig_core::EntityId>().is_ok(),
            _ => false,
        })
    })
}

/// Parses one typed identity retained by a prior agent branch.
///
/// - `name`: Human-readable identity kind for diagnostics.
/// - `value`: Durable string identity.
///
/// The typed identity.
///
/// Returns an invalid-request error when the durable identity is malformed.
fn parse_workspace_handle<T: std::str::FromStr>(
    name: &str,
    value: &str,
) -> Result<T, ApplicationError>
where
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|error| ApplicationError::InvalidRequest {
            operation: "agent_continue".to_owned(),
            message: format!("invalid durable {name} reference: {error}"),
        })
}

/// Finds one value in a durable branch result handle set.
///
/// - `handles`: Stable branch result handles.
/// - `kind`: Handle prefix to find.
///
/// The value after `kind:` when present.
fn handle_value<'a>(handles: &'a [String], kind: &str) -> Option<&'a str> {
    handles
        .iter()
        .find_map(|handle| handle.strip_prefix(&format!("{kind}:")))
}

/// Converts an exact workspace scope into durable result references.
///
/// - `selection`: Prior project context to preserve on the continuation result.
///
/// Stable project, snapshot, and root handles, or an empty set without scope.
/// # Returns
/// The operation result.
fn selection_handles(selection: Option<&AgentSelection>) -> Vec<String> {
    selection.map_or_else(Vec::new, |selection| {
        let mut handles = vec![
            format!("snapshot:{}", selection.snapshot),
            format!("root:{}", selection.root),
            format!("entity:{}", selection.entity),
        ];
        if let Some(project) = selection.project {
            handles.push(format!("project:{project}"));
        }
        handles
    })
}
