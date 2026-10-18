//! Composes the native TUI with the shared provider and agent orchestration runtime.

use std::sync::Arc;

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{ProviderAdapter, ProviderError};
use scriberig_agent_runtime::{AgentError, AgentRuntime, ChildExecutionScope};
use scriberig_app::{
    AgentCommandRequest, AgentSelection, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationService, SessionBackend, SessionRequest,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;
use crate::child_runtime::{LocalChildExecutor, inspect_catalog};
use crate::native_provider::NativeProvider;
#[path = "native_tui_agent/workspace.rs"]
mod workspace;
use workspace::workspace_from_result;
#[path = "native_tui_agent/model_selection.rs"]
mod model_selection;
#[path = "native_tui_agent/provider_config.rs"]
mod provider_config;
use model_selection::selected_runtime_model;

#[cfg(test)]
#[path = "native_tui_agent_tests.rs"]
mod tests;

/// Runs provider-backed agent commands submitted through the native TUI.
pub(super) struct NativeTuiAgentRunner {
    /// Configured provider, absent when local provider configuration is missing.
    provider: Option<NativeProvider>,
    /// Model identifier selected by the local runtime configuration.
    model: String,
    /// Safe models and selectable profiles exposed to the application service.
    model_library: scriberig_app::ModelLibrary,
    /// Shared project store used by application-backed agent tools.
    project_store: Arc<SqliteStore>,
}

impl NativeTuiAgentRunner {
    /// Builds the TUI agent runtime from trusted process configuration.
    ///
    /// # Parameters
    /// - `project_store`: Shared durable project and semantic store.
    /// - `super`: Parent module containing the native runtime composition.
    ///
    /// # Returns
    /// A runner configured from the `SCRIBE_AGENT_*` environment variables.
    ///
    /// # Errors
    /// Returns a structured provider configuration failure.
    pub(super) fn from_environment(project_store: Arc<SqliteStore>) -> Result<Self, ProviderError> {
        let model = std::env::var("SCRIBE_AGENT_MODEL").unwrap_or_else(|_| "default".to_owned());
        let provider_kind = std::env::var("SCRIBE_AGENT_PROVIDER")
            .unwrap_or_else(|_| "openai_compatible".to_owned());
        let catalog = provider_config::catalog::from_environment(&model)?;
        let provider = provider_config::from_environment(&provider_kind, &model, &catalog)?;
        Ok(Self {
            provider,
            model,
            model_library: catalog.library,
            project_store,
        })
    }

    /// Returns the safe model library installed in the application service.
    ///
    /// # Returns
    /// Configured model descriptors and selectable profiles.
    pub(super) fn model_library(&self) -> scriberig_app::ModelLibrary {
        self.model_library.clone()
    }
}

impl<B: ApplicationBackend> scriberig_tui::AgentCommandRunner<B> for NativeTuiAgentRunner {
    /// Executes one TUI agent request through the trusted shared runtime.
    ///
    /// # Parameters
    /// - `self`: Native provider and project-tool composition.
    /// - `service`: Shared application service owning durable sessions.
    /// - `context`: Trusted capabilities and semantic/bootstrap scope.
    /// - `session`: Session receiving the user instruction.
    /// - `branch`: Optional branch receiving the user instruction.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// Stable operation, session, and branch handles after runtime completion.
    ///
    /// # Errors
    /// Returns an authorization, provider, tool, or application failure.
    fn submit(
        &self,
        service: &ApplicationService<B>,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<scriberig_tui::AgentHandle, ApplicationError> {
        let selection = previous_workspace_selection(service, context, session)?;
        let request = AgentCommandRequest {
            session,
            branch,
            prompt: prompt.to_owned(),
            selection: selection.clone(),
        };
        let handle = service.begin_agent_execution(context, &request)?;
        let (model, thinking_level) =
            selected_runtime_model(service, context, session, &self.model)?;
        let cancelled = service.operations().cancellation_flag(&handle.operation)?;
        let Some(provider) = self.provider.as_ref() else {
            let message = "configure SCRIBE_AGENT_BASE_URL before starting agent work";
            service.fail_local_agent(&handle, message)?;
            return Err(ApplicationError::UnsupportedOperation {
                operation: message.to_owned(),
            });
        };
        let runtime_context = selection.as_ref().map_or_else(
            || context.clone(),
            |selection| context.clone().with_snapshot(selection.snapshot),
        );
        let result = execute_agent(
            provider.clone(),
            &model,
            thinking_level.as_deref(),
            service,
            &self.project_store,
            &runtime_context,
            session,
            handle.root_branch,
            handle.operation.clone(),
            prompt,
            selection.as_ref(),
            &cancelled,
        );
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let message = error.to_string();
                service.fail_local_agent(&handle, &message)?;
                return Err(ApplicationError::OperationFailed {
                    operation: handle.operation.to_string(),
                    message,
                });
            }
        };
        let workspace = workspace_from_result(&result)?;
        let mut result_handles = crate::child_runtime::compact_result_handles(&result);
        let workspace_handles = workspace.as_ref().map_or_else(
            || selection_handles(selection.as_ref()),
            |workspace| {
                vec![
                    format!("project:{}", workspace.project),
                    format!("snapshot:{}", workspace.snapshot),
                    format!("root:{}", workspace.root),
                    format!("manifest:{}", workspace.manifest_id),
                ]
            },
        );
        for handle in workspace_handles {
            if !result_handles.contains(&handle) {
                result_handles.push(handle);
            }
        }
        service.complete_local_agent_with_evidence(
            &handle,
            result.output,
            result_handles,
            result.tool_evidence,
            result.model_requests,
        )?;
        Ok(scriberig_tui::AgentHandle {
            operation: handle.operation,
            session,
            branch: Some(handle.root_branch),
            workspace,
        })
    }
}

/// Resolves the latest durable project scope produced in a session.
///
/// # Parameters
/// - `service`: Shared application boundary owning durable session state.
/// - `context`: Trusted authorization context for session reads.
/// - `session`: Session receiving the next user instruction.
///
/// # Returns
/// Exact project, root, selected entity, and snapshot when the session has an
/// onboarding result.
///
/// # Errors
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
    Ok(Some(AgentSelection {
        project: Some(parse_workspace_handle("project", project)?),
        root: parse_workspace_handle("root", root)?,
        entity: parse_workspace_handle("root", root)?,
        snapshot: parse_workspace_handle("snapshot", snapshot)?,
    }))
}

/// Parses one typed identity retained by a prior agent branch.
///
/// # Parameters
/// - `name`: Human-readable identity kind for diagnostics.
/// - `value`: Durable string identity.
///
/// # Returns
/// The typed identity.
///
/// # Errors
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
/// # Parameters
/// - `handles`: Stable branch result handles.
/// - `kind`: Handle prefix to find.
///
/// # Returns
/// The value after `kind:` when present.
fn handle_value<'a>(handles: &'a [String], kind: &str) -> Option<&'a str> {
    handles
        .iter()
        .find_map(|handle| handle.strip_prefix(&format!("{kind}:")))
}

/// Converts an exact workspace scope into durable result references.
///
/// # Parameters
/// - `selection`: Prior project context to preserve on the continuation result.
///
/// # Returns
/// Stable project, snapshot, and root handles, or an empty set without scope.
fn selection_handles(selection: Option<&AgentSelection>) -> Vec<String> {
    selection.map_or_else(Vec::new, |selection| {
        let mut handles = vec![
            format!("snapshot:{}", selection.snapshot),
            format!("root:{}", selection.root),
        ];
        if let Some(project) = selection.project {
            handles.push(format!("project:{project}"));
        }
        handles
    })
}

/// Executes one model loop with the shared semantic tool and child-branch adapters.
///
/// # Parameters
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
/// # Returns
/// Provider-neutral runtime output and tool observations.
///
/// # Errors
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
    if let Some(semantic_scope) = semantic_scope {
        let compiled = scriberig_agent_runtime::context::ContextCompiler::new(
            scriberig_agent_runtime::context::ContextPolicy::default(),
        )
        .compile(
            &scriberig_agent_runtime::context::BranchExecutionContext::new(
                session.to_string(),
                branch.to_string(),
                "semantic project continuation",
                prompt,
            )
            .with_project_scope(semantic_scope),
        );
        model_provider = model_provider.with_compiled_context(compiled);
    }
    let mut runtime = AgentRuntime::new(
        model_provider,
        LocalAgentTool::new(service).with_project_store(Arc::clone(project_store)),
        32,
    )
    .with_observed_operation(service.operations().clone(), operation)
    .with_branch_id(branch.to_string());
    runtime.run_with_child_executor_at(&scope, context, prompt, cancelled, &mut children)
}
