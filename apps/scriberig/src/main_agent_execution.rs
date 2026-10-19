//! Composes the provider-neutral model loop for the shared CLI runtime.

use std::sync::Arc;

use scriberig_agent_runtime::ChildExecutionScope;
use scriberig_agent_runtime::model_provider::ProviderModel;
use scriberig_agent_runtime::provider::ProviderAdapter;
use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_store_sqlite::SqliteStore;

use crate::agent_tool::LocalAgentTool;
use crate::child_runtime::{LocalChildExecutor, inspect_catalog};

/// Runs one provider-neutral model through the shared runtime loop.
///
/// - `provider`: Model provider adapter.
/// - `model`: Provider-native model identifier.
/// - `service`: Shared application operation boundary.
/// - `project_store`: Durable project store for application-backed tools.
/// - `context`: Trusted application context.
/// - `scope`: Durable branch identity for this runtime.
/// - `prompt`: Human instruction.
/// - `cancellation`: Shared cancellation flag.
/// - `output_callback`: Optional streamed text observer.
///
/// Completed runtime output and tool observations.
///
/// Returns the runtime's structured failure.
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_model<P: ProviderAdapter + Clone, B: ApplicationBackend>(
    provider: P,
    model: String,
    service: &ApplicationService<B>,
    project_store: &Arc<SqliteStore>,
    context: &ApplicationContext,
    scope: &ChildExecutionScope,
    prompt: &str,
    cancellation: &std::sync::atomic::AtomicBool,
    output_callback: Option<scriberig_agent_runtime::OutputCallback>,
) -> Result<scriberig_agent_runtime::AgentResult, scriberig_agent_runtime::AgentError> {
    let catalog = inspect_catalog();
    let cache = scriberig_agent_runtime::cache::ModelCache::new();
    let mut children = LocalChildExecutor::new(
        service,
        provider.clone(),
        model.clone(),
        cache.clone(),
        Arc::clone(project_store),
    );
    let runtime = scriberig_agent_runtime::AgentRuntime::new(
        ProviderModel::new(provider, model)
            .with_catalog(catalog)
            .with_capabilities(context.capabilities.clone())
            .with_cache(
                cache,
                scriberig_agent_runtime::cache::CachePolicy::ReuseExact,
            )
            .with_cache_namespace(scope.parent_branch.to_string()),
        LocalAgentTool::new(service).with_project_store(Arc::clone(project_store)),
        8,
    );
    let mut runtime = if let Some(callback) = output_callback {
        runtime.with_output_callback(callback)
    } else {
        runtime
    };
    runtime.run_with_child_executor_at(scope, context, prompt, cancellation, &mut children)
}
