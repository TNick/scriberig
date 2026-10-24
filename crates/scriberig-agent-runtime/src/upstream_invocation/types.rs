//! Defines stable branch invocation inputs, outputs, and failures.

use std::sync::Arc;

use rpi_ai::{AppliedRequestSettingsCallback, Model, Provider, RequestSecret};
use scriberig_core::EntityId;

use crate::context::CompiledContext;
use crate::upstream_tool::SemanticAgentTool;

/// Binds an upstream provider/model and approved tools to one `ScribeRig` branch.
///
/// The provider, model, compiled context, and tools are exact inputs for one
/// branch-local run. No upstream agent or session identity becomes canonical.
///
/// # Public members
/// - `branch`: `ScribeRig` branch identity for result correlation.
/// - `provider`: Upstream provider selected for this invocation.
/// - `model`: Exact upstream model selected for this invocation.
/// - `request_secret`: Optional opaque credential for protected provider calls.
/// - `bearer`: Whether a protected credential uses bearer authorization.
/// - `applied_settings`: Optional sink for sanitized final request settings.
/// - `thinking_level`: Upstream thinking level selected for this invocation.
/// - `context`: `ScribeRig`-compiled context for this branch.
/// - `tools`: Unique semantic adapters built from this branch's capability-
///   filtered tool catalog.
/// - `activity`: Shared `ScribeRig` activity sequence ordering this branch's
///   captured model requests and settled tool calls.
///
/// # Private members
/// None.
pub struct UpstreamBranchInvocation {
    /// `ScribeRig` branch identity for result correlation.
    pub branch: EntityId,
    /// Upstream provider selected for this invocation.
    pub provider: Arc<dyn Provider>,
    /// Exact upstream model selected for this invocation.
    pub model: Model,
    /// Optional opaque credential for protected provider calls.
    pub request_secret: Option<RequestSecret>,
    /// Whether a protected credential uses bearer authorization.
    pub bearer: bool,
    /// Optional sink for typed, sanitized final request settings.
    pub applied_settings: Option<AppliedRequestSettingsCallback>,
    /// Safe `ScribeRig` provider profile used for request provenance.
    pub provider_profile: Option<String>,
    /// Upstream thinking level selected for this invocation.
    pub thinking_level: rpi_ai::ThinkingLevel,
    /// `ScribeRig`-compiled context for this branch.
    pub context: CompiledContext,
    /// Branch-approved `ScribeRig` semantic tools only. The concrete type makes
    /// upstream plugin adapters unrepresentable here until an isolated host
    /// can mediate and constrain their process capabilities.
    pub tools: Vec<Arc<SemanticAgentTool>>,
    /// Shared `ScribeRig` ordering identity for this branch's execution
    /// activities. Request capture and tool settlement draw from one sequence
    /// so durable activity order reflects real execution order.
    pub activity: Arc<crate::orchestration::ActivitySequence>,
}

/// Compact output from one upstream invocation on a `ScribeRig` branch.
///
/// The upstream transcript is discarded after loop mechanics finish.
///
/// # Public members
/// - `branch`: `ScribeRig` branch identity supplied by the caller.
/// - `output`: Final assistant text projected for `ScribeRig` persistence.
/// - `model_requests`: Bounded provider-neutral request projections.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamBranchResult {
    /// `ScribeRig` branch identity supplied by the caller.
    pub branch: EntityId,
    /// Final assistant text projected for `ScribeRig` persistence.
    pub output: String,
    /// Bounded request projections captured before serialization.
    pub model_requests: Vec<scriberig_session::ModelRequestProjection>,
}

/// Failure to execute one branch through upstream `rpi-agent`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpstreamBranchError {
    /// The upstream agent loop returned an ordinary failure.
    Agent(String),
    /// The upstream provider rejected authentication or authorization.
    Authentication(String),
    /// The branch tool set is mis-scoped or contains an invalid name.
    InvalidToolSet,
    /// The branch cancellation token was cancelled.
    Cancelled,
    /// The upstream loop ended without a final assistant message.
    MissingFinalAssistant,
}

/// Failure paired with any safe requests captured before settlement failed.
///
/// # Public members
/// - `error`: Safe upstream execution failure.
/// - `model_requests`: Requests that were prepared before the failure.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamBranchFailure {
    /// Safe upstream execution failure.
    pub error: UpstreamBranchError,
    /// Requests prepared before the failure.
    pub model_requests: Vec<scriberig_session::ModelRequestProjection>,
}

impl std::fmt::Display for UpstreamBranchError {
    /// Formats a safe branch invocation failure.
    ///
    /// # Parameters
    /// - `self`: Failure to format.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Agent(message) => write!(formatter, "upstream agent: {message}"),
            Self::Authentication(message) => {
                write!(formatter, "upstream authentication: {message}")
            }
            Self::InvalidToolSet => formatter.write_str("branch tool set has invalid tool names"),
            Self::Cancelled => formatter.write_str("branch invocation cancelled"),
            Self::MissingFinalAssistant => {
                formatter.write_str("upstream agent returned no final assistant message")
            }
        }
    }
}

impl std::error::Error for UpstreamBranchError {}

impl From<UpstreamBranchError> for crate::AgentError {
    /// Projects one branch invocation failure onto the public runtime category.
    ///
    /// Authentication and cancellation stay distinct from an ordinary provider
    /// failure so callers never have to classify provider transport text.
    ///
    /// # Parameters
    /// - `error`: Branch invocation failure to project.
    ///
    /// # Returns
    /// The public `ScribeRig` agent-runtime failure.
    fn from(error: UpstreamBranchError) -> Self {
        match error {
            UpstreamBranchError::Authentication(message) => Self::Authentication(message),
            UpstreamBranchError::Cancelled => Self::Cancelled,
            error => Self::Provider(error.to_string()),
        }
    }
}
