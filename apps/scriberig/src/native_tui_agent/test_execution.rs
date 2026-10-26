//! Runs deterministic test providers through the production upstream loop.

use std::sync::Arc;

use scriberig_agent_runtime::AgentError;
use scriberig_app::{AgentSelection, ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

use super::{NativeScenario, upstream_execution};

/// Executes a deterministic scenario through the shared upstream branch path.
///
/// # Parameters
/// - `provider`: Deterministic `ScribeRig` scenario provider.
/// - `model`: Model identity exposed to the upstream agent.
/// - `_provider_id`: Legacy fixture identity retained at the test call site.
/// - `_thinking_level`: Legacy fixture setting not used by deterministic models.
/// - `service`: Shared application and session boundary.
/// - `project_store`: Durable project catalog and candidate store.
/// - `context`: Trusted capabilities and operation scope.
/// - `session`: Canonical session identity.
/// - `branch`: Exact root branch receiving the turn.
/// - `_operation`: Operation identity created by the fixture.
/// - `prompt`: User-authored request.
/// - `selection`: Exact project selection, when present.
/// - `cancelled`: Shared operation cancellation flag.
/// - `model_requests_on_failure`: Destination for captured request projections.
///
/// # Returns
/// Final branch output, tool evidence, and bounded request projections.
///
/// # Errors
/// Returns a safe upstream provider or execution failure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_agent<B: ApplicationBackend + Send + Sync + 'static>(
    provider: NativeScenario,
    model: &str,
    _provider_id: Option<&str>,
    _thinking_level: Option<&str>,
    service: &ApplicationService<B>,
    project_store: &Arc<SqliteStore>,
    context: &ApplicationContext,
    session: EntityId,
    branch: EntityId,
    _operation: scriberig_app::OperationId,
    prompt: &str,
    selection: Option<&AgentSelection>,
    cancelled: &std::sync::atomic::AtomicBool,
    model_requests_on_failure: &mut Vec<scriberig_session::ModelRequestProjection>,
) -> Result<scriberig_agent_runtime::AgentResult, AgentError> {
    let model = rpi_ai::Model::new(
        model,
        model,
        rpi_ai::Api::OpenaiCompletions,
        "local",
        "http://scenario.invalid",
    );
    let provider = provider.into_provider(model.clone());
    upstream_execution::execute_root_branch_with_options(
        service,
        context,
        session,
        branch,
        prompt,
        selection,
        Arc::clone(project_store),
        provider,
        model,
        None,
        None,
        cancelled,
        None,
        None,
    )
    .map_err(
        |failure: upstream_execution::UpstreamBranchExecutionFailure| {
            *model_requests_on_failure = failure.failure.model_requests;
            failure.failure.error.into()
        },
    )
}
