//! Composes the native TUI with the shared provider and agent orchestration runtime.

use std::sync::Arc;

use scriberig_agent_runtime::AgentMessage;
use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    CredentialSource, ProviderAdapter, ProviderConfig, ProviderError,
};
use scriberig_agent_runtime::provider::{ModelRequest, ModelResponse};
use scriberig_agent_runtime::{AgentError, AgentRuntime, ChildExecutionScope};
use scriberig_app::{
    AgentCommandRequest, ApplicationBackend, ApplicationContext, ApplicationError,
    ApplicationService,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;
use crate::child_runtime::{LocalChildExecutor, inspect_catalog};

/// Runs provider-backed agent commands submitted through the native TUI.
pub(super) struct NativeTuiAgentRunner {
    /// Configured provider, absent when local provider configuration is missing.
    provider: Option<NativeProvider>,
    /// Model identifier selected by the local runtime configuration.
    model: String,
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
        let provider = match provider_kind.as_str() {
            "deterministic" => Some(NativeProvider::Deterministic(
                crate::deterministic_provider::BootstrapProvider,
            )),
            "openai_compatible" => std::env::var("SCRIBE_AGENT_BASE_URL")
                .ok()
                .map(|base_url| {
                    let credential = std::env::var("SCRIBE_AGENT_API_KEY_ENV")
                        .map_or(CredentialSource::None, CredentialSource::Environment);
                    let config = ProviderConfig::new("local", base_url, model.clone(), credential);
                    scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider::new(
                        config,
                    )
                    .map(NativeProvider::OpenAiCompatible)
                })
                .transpose()?,
            unknown => {
                return Err(ProviderError::Configuration(format!(
                    "unsupported SCRIBE_AGENT_PROVIDER: {unknown}"
                )));
            }
        };
        Ok(Self {
            provider,
            model,
            project_store,
        })
    }
}

/// Selects the available provider adapter for the local TUI runtime.
#[derive(Clone)]
enum NativeProvider {
    /// Remote OpenAI-compatible model endpoint.
    OpenAiCompatible(scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider),
    /// Deterministic onboarding scenario used by product tests.
    Deterministic(crate::deterministic_provider::BootstrapProvider),
}

impl ProviderAdapter for NativeProvider {
    /// Completes through the selected remote or deterministic adapter.
    ///
    /// # Parameters
    /// - `self`: Selected native provider.
    /// - `request`: Normalized provider request.
    ///
    /// # Returns
    /// The normalized model response.
    ///
    /// # Errors
    /// Returns a classified provider failure.
    fn complete(&self, request: &ModelRequest) -> Result<ModelResponse, ProviderError> {
        match self {
            Self::OpenAiCompatible(provider) => provider.complete(request),
            Self::Deterministic(provider) => provider.complete(request),
        }
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
        let request = AgentCommandRequest {
            session,
            branch,
            prompt: prompt.to_owned(),
            selection: None,
        };
        let handle = service.begin_agent_execution(context, &request)?;
        let Some(provider) = self.provider.as_ref() else {
            let message = "configure SCRIBE_AGENT_BASE_URL before starting agent work";
            service.fail_local_agent(&handle, message)?;
            return Err(ApplicationError::UnsupportedOperation {
                operation: message.to_owned(),
            });
        };
        let result = execute_agent(
            provider.clone(),
            &self.model,
            service,
            &self.project_store,
            context,
            session,
            handle.root_branch,
            prompt,
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
        let result_handles = workspace.as_ref().map_or_else(Vec::new, |workspace| {
            vec![
                format!("project:{}", workspace.project),
                format!("snapshot:{}", workspace.snapshot),
                format!("root:{}", workspace.root),
                format!("manifest:{}", workspace.manifest_id),
            ]
        });
        service.complete_local_agent_with_handles(&handle, result.output, result_handles)?;
        Ok(scriberig_tui::AgentHandle {
            operation: handle.operation,
            session,
            branch: Some(handle.root_branch),
            workspace,
        })
    }
}

/// Extracts the exact workspace returned by the project workspace tool.
///
/// # Parameters
/// - `result`: Completed runtime transcript.
///
/// # Returns
/// The returned project workspace when the agent activated one.
///
/// # Errors
/// Returns an internal error when a workspace tool result omits valid references.
fn workspace_from_result(
    result: &scriberig_agent_runtime::AgentResult,
) -> Result<Option<scriberig_tui::AgentWorkspace>, ApplicationError> {
    let Some(tool) = result
        .messages
        .iter()
        .rev()
        .find_map(|message| match message {
            AgentMessage::Tool(tool)
                if tool.object_type.as_deref() == Some("project_workspace") =>
            {
                Some(tool)
            }
            _ => None,
        })
    else {
        return Ok(None);
    };
    let manifest_id = result
        .messages
        .iter()
        .find_map(|message| match message {
            AgentMessage::Tool(tool) if tool.object_type.as_deref() == Some("project_scan") => {
                tool.handles.get(1).cloned()
            }
            _ => None,
        })
        .ok_or_else(|| ApplicationError::Internal {
            message: "project workspace has no reviewed manifest reference".into(),
        })?;
    let [project, snapshot, root] = tool.handles.as_slice() else {
        return Err(ApplicationError::Internal {
            message: "project workspace result must contain project, snapshot, and root".into(),
        });
    };
    let parse_error = |message: &str| ApplicationError::Internal {
        message: message.to_owned(),
    };
    let project = project
        .parse()
        .map_err(|error| parse_error(&format!("invalid workspace project: {error}")))?;
    let snapshot = snapshot
        .parse()
        .map_err(|error| parse_error(&format!("invalid workspace snapshot: {error}")))?;
    let root = root
        .parse()
        .map_err(|error| parse_error(&format!("invalid workspace root: {error}")))?;
    let name = tool
        .summary
        .as_deref()
        .and_then(|summary| summary.strip_prefix("active semantic workspace "))
        .and_then(|summary| summary.split_once(" at snapshot ").map(|(name, _)| name))
        .ok_or_else(|| parse_error("project workspace result omitted its name"))?;
    Ok(Some(scriberig_tui::AgentWorkspace {
        project,
        name: name.to_owned(),
        snapshot,
        root,
        manifest_id,
    }))
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
    service: &ApplicationService<B>,
    project_store: &Arc<SqliteStore>,
    context: &ApplicationContext,
    session: EntityId,
    branch: EntityId,
    prompt: &str,
) -> Result<scriberig_agent_runtime::AgentResult, AgentError>
where
    P: ProviderAdapter + Clone,
    B: ApplicationBackend,
{
    let cache = ModelCache::new();
    let scope = ChildExecutionScope {
        session,
        parent_branch: branch,
        depth: 0,
        active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let mut children = LocalChildExecutor {
        service,
        provider: provider.clone(),
        model: model.to_owned(),
        cache: cache.clone(),
        project_store: Arc::clone(project_store),
    };
    let mut runtime = AgentRuntime::new(
        ProviderModel::new(provider, model)
            .with_catalog(inspect_catalog())
            .with_capabilities(context.capabilities.clone())
            .with_cache(cache, CachePolicy::ReuseExact)
            .with_cache_namespace(branch.to_string()),
        LocalAgentTool::new(service).with_project_store(Arc::clone(project_store)),
        8,
    );
    runtime.run_with_child_executor_at(
        &scope,
        context,
        prompt,
        &std::sync::atomic::AtomicBool::new(false),
        &mut children,
    )
}
