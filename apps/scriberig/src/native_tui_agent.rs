//! Composes the native TUI with the shared provider and agent orchestration runtime.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{ProviderAdapter, ProviderError};
use scriberig_agent_runtime::{AgentError, AgentRuntime, ChildExecutionScope};
use scriberig_app::{
    AgentSelection, ApplicationBackend, ApplicationContext, ApplicationError, ApplicationService,
    SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;
use crate::child_runtime::{LocalChildExecutor, inspect_catalog};
use crate::native_provider::NativeProvider;
#[path = "native_tui_agent/context.rs"]
mod context;
#[path = "native_tui_agent/workspace.rs"]
mod workspace;
use workspace::workspace_from_result;
#[path = "native_tui_agent/model_selection.rs"]
mod model_selection;
#[path = "native_tui_agent/plan_worker.rs"]
mod plan_worker;
#[path = "native_tui_agent/provider_config.rs"]
mod provider_config;
#[path = "native_tui_agent/runner.rs"]
mod runner;
use model_selection::selected_runtime_model;
use plan_worker::NativePlanWorkerExecutor;

#[cfg(test)]
#[path = "native_tui_agent_tests.rs"]
mod tests;

/// Runs provider-backed agent commands submitted through the native TUI.
pub(super) struct NativeTuiAgentRunner {
    /// Configured adapters keyed by the model library's safe connection IDs.
    providers: std::collections::BTreeMap<String, NativeProvider>,
    /// Model identifier selected by the local runtime configuration.
    model: String,
    /// Safe models and selectable profiles exposed to the application service.
    model_library: scriberig_app::ModelLibrary,
    /// Upstream catalog retained as the model selection authority.
    upstream_models: Vec<rpi_ai::Model>,
    /// Shared project store used by application-backed agent tools.
    project_store: Arc<SqliteStore>,
    /// In-process guard against starting two coordinator loops for one plan.
    active_coordinators: Mutex<HashSet<EntityId>>,
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
    pub(super) fn from_environment(project_store: Arc<SqliteStore>) -> Result<Self, ProviderError> {
        let provider_kind = std::env::var("SCRIBE_AGENT_PROVIDER")
            .unwrap_or_else(|_| "openai_compatible".to_owned());
        let shared_config = if provider_kind == "openai_compatible"
            && std::env::var_os("SCRIBE_AGENT_BASE_URL").is_none()
        {
            crate::agent_config::resolve_provider_config()
                .map_err(|error| ProviderError::Configuration(error.message))?
                .map(|(config, _)| config)
        } else {
            None
        };
        let configured_model = shared_config
            .as_ref()
            .map_or("default", |config| config.model.as_str());
        let model =
            std::env::var("SCRIBE_AGENT_MODEL").unwrap_or_else(|_| configured_model.to_owned());
        let catalog = provider_config::catalog::from_environment(&model)?;
        let mut providers = std::collections::BTreeMap::new();
        if let Some(provider) =
            provider_config::from_environment(&provider_kind, &model, shared_config)?
        {
            providers.insert("local".to_owned(), provider);
        }
        for connection in &catalog.connections {
            let provider = provider_config::from_connection(connection, &model)?;
            providers.insert(connection.id.clone(), provider);
        }
        Ok(Self {
            providers,
            model,
            model_library: catalog.library,
            upstream_models: catalog.models,
            project_store,
            active_coordinators: Mutex::new(HashSet::new()),
        })
    }

    /// Returns the safe model library installed in the application service.
    ///
    /// Configured model descriptors and selectable profiles.
    /// # Returns
    /// The operation result.
    pub(super) fn model_library(&self) -> scriberig_app::ModelLibrary {
        self.model_library.clone()
    }

    /// Returns the upstream catalog used for model resolution.
    ///
    /// # Returns
    /// A copy of the upstream model catalog.
    pub(super) fn upstream_models(&self) -> Vec<rpi_ai::Model> {
        self.upstream_models.clone()
    }
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
        ["project", "snapshot", "root"]
            .iter()
            .all(|kind| handle_value(handles, kind).is_some())
            .then_some(handles)
    }) else {
        return Ok(None);
    };
    let project = handle_value(handles, "project").expect("complete workspace handle set");
    let snapshot = handle_value(handles, "snapshot").expect("complete workspace handle set");
    let root = handle_value(handles, "root").expect("complete workspace handle set");
    let entity = handle_value(handles, "entity").unwrap_or(root);
    Ok(Some(AgentSelection {
        project: Some(parse_workspace_handle("project", project)?),
        root: parse_workspace_handle("root", root)?,
        entity: parse_workspace_handle("entity", entity)?,
        snapshot: parse_workspace_handle("snapshot", snapshot)?,
    }))
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

/// Executes one model loop with the shared semantic tool and child-branch adapters.
///
/// - `super`: Parent executable composition module.
/// - `provider`: Cloneable provider adapter for parent and child branches.
/// - `model`: Configured model identifier.
/// - `service`: Shared application operation boundary.
/// - `project_store`: Shared project catalog and artifact store.
/// - `context`: Trusted request capabilities and scope.
/// - `session`: Durable session receiving branch activity.
/// - `branch`: Exact root branch executing this turn.
/// - `prompt`: User-authored instruction.
///
/// Provider-neutral runtime output and tool observations.
///
/// Returns a bounded runtime, provider, or tool failure.
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_agent<P, B>(
    provider: P,
    model: &str,
    thinking_level: Option<&str>,
    service: &ApplicationService<B>,
    project_store: &Arc<SqliteStore>,
    context: &ApplicationContext,
    session: EntityId,
    branch: EntityId,
    operation: scriberig_app::OperationId,
    prompt: &str,
    selection: Option<&AgentSelection>,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<scriberig_agent_runtime::AgentResult, AgentError>
where
    P: ProviderAdapter + Clone,
    B: ApplicationBackend,
{
    let cache = ModelCache::new();
    let semantic_scope =
        selection.map(
            |selection| scriberig_agent_runtime::context::ProjectSemanticScope {
                project: selection
                    .project
                    .map_or_else(|| "unknown".to_owned(), |project| project.to_string()),
                snapshot: selection.snapshot.to_string(),
                root: selection.root.to_string(),
                selection: Some(selection.entity.to_string()),
            },
        );
    let scope = ChildExecutionScope {
        session,
        parent_branch: branch,
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        semantic_scope: semantic_scope.clone(),
    };
    let mut children = LocalChildExecutor {
        service,
        operation: Some(operation.clone()),
        provider: provider.clone(),
        model: model.to_owned(),
        thinking_level: thinking_level.map(str::to_owned),
        cache: cache.clone(),
        project_store: Arc::clone(project_store),
    };
    let mut model_provider = ProviderModel::new(provider, model)
        .with_thinking_level(thinking_level.map(str::to_owned))
        .with_catalog(inspect_catalog())
        .with_capabilities(context.capabilities.clone())
        .with_cache(cache, CachePolicy::ReuseExact)
        .with_cache_namespace(branch.to_string());
    let runtime_context = selection.map_or_else(
        || context.clone(),
        |selection| context.clone().with_snapshot(selection.snapshot),
    );
    let compiled = context::compile_branch_context(
        service,
        &runtime_context,
        session,
        branch,
        prompt,
        semantic_scope,
    )?;
    model_provider = model_provider.with_compiled_context(compiled);
    let mut runtime = AgentRuntime::new(
        model_provider,
        LocalAgentTool::new(service).with_project_store(Arc::clone(project_store)),
        32,
    )
    .with_observed_operation(service.operations().clone(), operation)
    .with_branch_id(branch.to_string());
    runtime.run_with_child_executor_at(&scope, context, prompt, cancelled, &mut children)
}
