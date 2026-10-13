//! Composes the native TUI with the shared provider and agent orchestration runtime.

use std::sync::Arc;

use scriberig_agent_runtime::cache::{CachePolicy, ModelCache};
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::{
    CredentialSource, ProviderAdapter, ProviderConfig, ProviderError,
};
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
    provider: Option<scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider>,
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
        let base_url = std::env::var("SCRIBE_AGENT_BASE_URL").ok();
        let provider = base_url
            .map(|base_url| {
                let credential = std::env::var("SCRIBE_AGENT_API_KEY_ENV")
                    .map_or(CredentialSource::None, CredentialSource::Environment);
                let config = ProviderConfig::new("local", base_url, model.clone(), credential);
                scriberig_agent_runtime::openai_compatible::OpenAiCompatibleProvider::new(config)
            })
            .transpose()?;
        Ok(Self {
            provider,
            model,
            project_store,
        })
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
        service.complete_local_agent(&handle, result.output)?;
        Ok(scriberig_tui::AgentHandle {
            operation: handle.operation,
            session,
            branch: Some(handle.root_branch),
        })
    }
}

/// Executes one model loop with the shared semantic tool and child-branch adapters.
///
/// # Parameters
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
fn execute_agent<P, B>(
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
