//! Implements provider-neutral agent orchestration above reusable agent tools.

use scriberig_core::EntityId;

include!(concat!(
    env!("OUT_DIR"),
    "/upstream_model_runtime_revision.rs"
));

pub mod context;
pub mod continuation;
pub mod coordinator;
pub mod host_configuration;
pub mod orchestration;
pub mod plan_context;
#[cfg(feature = "isolated-plugin-worker")]
pub mod plugin_worker;
pub mod profile;
pub mod protected_credential;
pub mod provider_readiness;
pub mod scheduler;
pub mod tools;
pub mod upstream_invocation;
pub mod upstream_tool;

/// Thread-safe sink for provider-neutral assistant text fragments.
pub type OutputCallback = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

/// Thread-safe sink for provider-visible reasoning-summary fragments.
///
/// The `u64` argument is a monotonic ordinal scoped to the current invocation
/// and reflects provider arrival order. Only upstream
/// [`rpi_ai::AssistantMessageEvent::ReasoningSummaryDelta`] text may be passed
/// here; raw thinking deltas and hidden chain-of-thought must never use it.
pub type ReasoningSummaryCallback = std::sync::Arc<dyn Fn(u64, &str) + Send + Sync>;

/// Trusted request to start a nested reasoning branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildRequest {
    /// Purpose assigned to the child branch.
    pub purpose: String,
    /// Bounded prompt inherited by the child.
    pub prompt: String,
    /// Capability identifiers explicitly delegated to the child.
    pub capabilities: Vec<String>,
    /// Compact result shape expected from the child.
    pub expected_result: String,
    /// Optional upstream execution assignment requested for the child.
    pub execution: Option<scriberig_session::ModelAssignment>,
}

/// Provider binding resolved for one explicit child execution assignment.
///
/// # Public members
/// - `provider`: Host-created upstream provider.
/// - `model`: Exact upstream model validated against the current catalog.
/// - `request_secret`: Optional credential authorized for this model.
/// - `bearer`: Whether the credential uses bearer authorization.
/// - `thinking_level`: Upstream-supported thinking level for the child.
/// - `provider_profile`: Safe connection identity used for provenance.
///
/// # Private members
/// None.
pub struct ChildProviderBinding {
    /// Host-created upstream provider.
    pub provider: std::sync::Arc<dyn rpi_ai::Provider>,
    /// Exact upstream model validated against the current catalog.
    pub model: rpi_ai::Model,
    /// Optional credential authorized for this model.
    pub request_secret: Option<rpi_ai::RequestSecret>,
    /// Whether the credential uses bearer authorization.
    pub bearer: bool,
    /// Upstream-supported thinking level for the child.
    pub thinking_level: rpi_ai::ThinkingLevel,
    /// Safe connection identity used for provenance.
    pub provider_profile: Option<String>,
}

/// Resolves a Scriberig assignment into a host-authorized upstream binding.
pub type ChildProviderResolver = std::sync::Arc<
    dyn Fn(&scriberig_session::ModelAssignment) -> Result<ChildProviderBinding, String>
        + Send
        + Sync,
>;

/// Identifies the durable parent branch for delegated work.
#[derive(Clone)]
pub struct ChildExecutionScope {
    /// Session containing the parent branch.
    pub session: EntityId,
    /// Parent branch receiving the child result reference.
    pub parent_branch: EntityId,
    /// Zero-based nested branch depth of the parent.
    pub depth: usize,
    /// Shared active-branch counter for the execution tree.
    pub active_branches: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// Exact semantic project scope inherited by this branch tree.
    pub semantic_scope: Option<context::ProjectSemanticScope>,
    /// Optional host resolver for child-specific provider/model assignments.
    pub provider_resolver: Option<ChildProviderResolver>,
}

/// Structured failure returned by the agent orchestration boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentError {
    /// Upstream provider or agent execution failed.
    Provider(String),
    /// The upstream provider rejected authentication or authorization.
    Authentication(String),
    /// The caller requested cancellation.
    Cancelled,
}

impl AgentError {
    /// Reports whether this failure is an upstream authentication failure.
    ///
    /// # Returns
    /// `true` for [`AgentError::Authentication`].
    #[must_use]
    pub fn is_authentication(&self) -> bool {
        matches!(self, Self::Authentication(_))
    }
}

impl std::fmt::Display for AgentError {
    /// Formats a safe agent-runtime failure.
    ///
    /// - `self`: Agent error.
    /// - `formatter`: Destination formatter.
    ///
    /// Formatting success or formatter failure.
    ///
    /// Returns the formatter error when output cannot be written.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider(message) => write!(formatter, "agent provider: {message}"),
            Self::Authentication(message) => {
                write!(formatter, "agent authentication: {message}")
            }
            Self::Cancelled => formatter.write_str("agent operation cancelled"),
        }
    }
}

impl std::error::Error for AgentError {}

/// Final result of one provider/tool execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentResult {
    /// Provider-produced final text.
    pub output: String,
    /// Executed tool calls with exact arguments and bounded structured results.
    pub tool_evidence: Vec<scriberig_app::ToolExecutionEvidence>,
    /// Bounded safe requests captured immediately before provider calls.
    pub model_requests: Vec<scriberig_session::ModelRequestProjection>,
}
