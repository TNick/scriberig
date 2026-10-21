//! Builds the configured upstream provider and executes one durable CLI branch.

use std::sync::Arc;

use rpi_ai::{Api, Model, Provider, RequestSecret};
use scriberig_agent_runtime::ChildExecutionScope;
use scriberig_app::{ApplicationBackend, ApplicationContext, ApplicationService};
use scriberig_store_sqlite::SqliteStore;

/// Provider failure with safe branch evidence captured before settlement.
///
/// # Public members
/// - `error`: Provider-neutral failure returned to the CLI.
/// - `model_requests`: Safe requests captured before failure.
/// - `tool_evidence`: Semantic tool observations captured before failure.
///
/// # Private members
/// None.
pub(crate) struct ComposedAgentFailure {
    /// Provider-neutral failure returned to the CLI.
    pub(crate) error: scriberig_agent_runtime::AgentError,
    /// Safe request projections observed before the failure.
    pub(crate) model_requests: Vec<scriberig_session::ModelRequestProjection>,
    /// Semantic tool observations completed before the failure.
    pub(crate) tool_evidence: Vec<scriberig_app::ToolExecutionEvidence>,
}

impl From<scriberig_agent_runtime::AgentError> for ComposedAgentFailure {
    /// Wraps setup and local-runtime failures without captured evidence.
    ///
    /// # Parameters
    /// - `error`: Failure without captured branch evidence.
    ///
    /// # Returns
    /// The failure with empty evidence collections.
    fn from(error: scriberig_agent_runtime::AgentError) -> Self {
        Self {
            error,
            model_requests: Vec::new(),
            tool_evidence: Vec::new(),
        }
    }
}

/// Runs one configured OpenAI-compatible model through the upstream agent loop.
///
/// The host resolves credentials against the exact configured model endpoint.
/// `ScribeRig` branch, semantic-tool, and child-result behavior is supplied by the
/// shared upstream execution broker.
///
/// # Parameters
/// - `args`: Resolved provider and prompt configuration.
/// - `service`: Shared application operation boundary.
/// - `project_store`: Durable project store for semantic tools.
/// - `context`: Trusted application capabilities for this invocation.
/// - `scope`: Canonical session and branch identity.
/// - `cancellation`: Shared `ScribeRig` cancellation flag.
/// - `output_callback`: Optional streamed provider-visible text observer.
///
/// # Returns
/// Completed output from the configured endpoint, exact extension model, or
/// upstream-backed deterministic Echo provider.
///
/// # Errors
/// Returns provider setup, credential authorization, semantic, budget, or
/// cancellation failures.
pub(crate) fn execute_model<B: ApplicationBackend + Sync>(
    args: &scriberig_cli::RunArgs,
    service: &ApplicationService<B>,
    project_store: &Arc<SqliteStore>,
    context: &ApplicationContext,
    scope: &ChildExecutionScope,
    cancellation: &std::sync::atomic::AtomicBool,
    output_callback: Option<scriberig_agent_runtime::OutputCallback>,
) -> Result<scriberig_agent_runtime::AgentResult, ComposedAgentFailure> {
    crate::native_tui_agent::validate_isolated_plugin_configuration()
        .map_err(|error| scriberig_agent_runtime::AgentError::Provider(error.to_string()))?;
    let (model, provider, request_secret) = if let Some(base_url) = &args.base_url {
        let model = configured_model(args, base_url);
        let provider = create_provider(&model)?;
        let request_secret = request_secret(args, &model)?;
        (model, provider, request_secret)
    } else {
        let model = Model::new(
            args.model.clone(),
            "ScribeRig Echo",
            Api::Faux,
            "scriberig-echo",
            "local://echo",
        );
        let provider = crate::native_tui_agent::echo_provider(model.clone());
        (model, provider, None)
    };
    let provider_id = model.provider.clone();
    let result = crate::native_tui_agent::upstream_execution::execute_root_branch_with_options(
        service,
        context,
        scope.session,
        scope.parent_branch,
        &args.prompt,
        None,
        Arc::clone(project_store),
        provider,
        model,
        request_secret,
        None,
        cancellation,
        Some(provider_id),
        output_callback,
    );
    result.map_err(|failure| {
        let error = match failure.failure.error {
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Cancelled => {
                scriberig_agent_runtime::AgentError::Cancelled
            }
            scriberig_agent_runtime::upstream_invocation::UpstreamBranchError::Authentication(
                message,
            ) => scriberig_agent_runtime::AgentError::Authentication(message),
            error => scriberig_agent_runtime::AgentError::Provider(error.to_string()),
        };
        ComposedAgentFailure {
            error,
            model_requests: failure.failure.model_requests,
            tool_evidence: failure.tool_evidence,
        }
    })
}

/// Creates a provider from the exact endpoint and model supplied by host config.
///
/// # Parameters
/// - `model`: Provider-qualified model descriptor for one CLI invocation.
///
/// # Returns
/// A built-in upstream provider bound to this model.
///
/// # Errors
/// Returns a safe provider-factory failure.
fn create_provider(
    model: &Model,
) -> Result<Arc<dyn Provider>, scriberig_agent_runtime::AgentError> {
    rpi_model_runtime::create_builtin_provider(
        &model.provider,
        vec![model.clone()],
        reqwest::Client::new(),
    )
    .map_err(|error| scriberig_agent_runtime::AgentError::Provider(error.to_string()))
}

/// Resolves one request credential after the configured endpoint is fixed.
///
/// # Parameters
/// - `args`: Resolved CLI credential reference and endpoint configuration.
/// - `model`: Exact selected upstream provider model.
///
/// # Returns
/// Opaque request-only material, or `None` for an explicitly unauthenticated
/// provider configuration.
///
/// # Errors
/// Returns a safe credential authorization or resolution failure.
fn request_secret(
    args: &scriberig_cli::RunArgs,
    model: &Model,
) -> Result<Option<RequestSecret>, scriberig_agent_runtime::AgentError> {
    let source = crate::agent_config::authorized_credential_provider(args)
        .map_err(|error| scriberig_agent_runtime::AgentError::Provider(error.to_string()))?;
    match source {
        None => Ok(None),
        Some(credentials) => {
            scriberig_agent_runtime::protected_credential::request_secret_for_model(
                credentials.as_ref(),
                model,
            )
            .map_err(|error| scriberig_agent_runtime::AgentError::Provider(error.to_string()))
        }
    }
}

/// Builds the exact provider model descriptor selected by CLI configuration.
///
/// # Parameters
/// - `args`: Resolved provider and model identifiers.
/// - `base_url`: Exact configured provider endpoint.
///
/// # Returns
/// A model descriptor bound to the `OpenAI Completions` protocol and endpoint.
fn configured_model(args: &scriberig_cli::RunArgs, base_url: &str) -> Model {
    Model::new(
        args.model.clone(),
        args.model.clone(),
        Api::OpenaiCompletions,
        args.provider.clone(),
        base_url.to_owned(),
    )
}
