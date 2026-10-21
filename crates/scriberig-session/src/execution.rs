//! Defines provider-neutral model execution records and deterministic activity order.

use scriberig_core::EntityId;

/// Session-scoped model configuration for future invocations.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SessionExecutionSelection {
    /// Monotonic mutation revision, independent from event cursors.
    pub revision: u64,
    /// Selected upstream provider identity.
    pub provider_id: Option<String>,
    /// Selected upstream provider-scoped model identity.
    pub model_id: Option<String>,
    /// Selected model-advertised thinking-level identity.
    pub thinking_level: Option<String>,
    /// Bounded replayable model and thinking selection provenance.
    #[serde(default)]
    pub history: Vec<SessionExecutionChange>,
}

/// Records one durable future-invocation selection change.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SessionExecutionChange {
    /// Selection mutation revision that produced this record.
    pub revision: u64,
    /// Semantic category of the changed selection.
    pub kind: AgentActivityKind,
    /// Safe concise description of the resulting selection.
    pub summary: String,
    /// Resolved upstream provider identity after the change, when selected.
    pub provider_id: Option<String>,
    /// Resolved upstream model identity after the change, when selected.
    pub model_id: Option<String>,
    /// Resolved thinking level after the change, when selected.
    pub thinking_level: Option<String>,
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, schemars::JsonSchema, serde::Deserialize, serde::Serialize,
)]
/// Provider-neutral execution activity kinds shared by persistence and events.
///
/// # Public members
/// Each variant identifies one shared semantic activity kind.
///
/// # Private members
/// None.
pub enum SessionActivityKind {
    /// Human supplied a prompt or instruction.
    HumanInput,
    /// Externally meaningful model output was committed.
    AssistantOutput,
    /// Safe user-facing reasoning summary.
    ReasoningSummary,
    /// Scriberig runtime phase or status.
    RuntimeStatus,
    /// Model request was issued.
    ModelInvocation,
    /// Safe captured model request detail.
    ModelRequest,
    /// A trusted tool was invoked.
    ToolInvocation,
    /// A tool returned or failed.
    ToolResult,
    /// A child branch was created.
    BranchStarted,
    /// A child branch completed.
    BranchCompleted,
    /// Operation progress changed.
    OperationProgress,
    /// Validation completed.
    Validation,
    /// Future model selection changed.
    ModelSelectionChanged,
    /// Future thinking-level selection changed.
    ThinkingLevelChanged,
    /// A durable plan was created.
    PlanCreated,
    /// A new durable plan revision was accepted.
    PlanRevised,
    /// A plan node became ready for worker execution.
    PlanNodeReady,
    /// A plan node acquired an isolated worker branch.
    PlanNodeStarted,
    /// A plan node completed with an explicit result.
    PlanNodeCompleted,
    /// A plan node failed and continues to block dependents.
    PlanNodeFailed,
    /// A plan assignment changed for a future invocation.
    AssignmentChanged,
    /// Coordinator ownership or lifecycle changed.
    CoordinatorChanged,
    /// A dependency edge changed fulfillment state.
    DependencyStateChanged,
    /// A typed semantic domain operation was recorded.
    SemanticOperation,
    /// An explicitly authorized external process ran.
    ExternalProcess,
    /// An operation or activity failed.
    Error,
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, schemars::JsonSchema, serde::Deserialize, serde::Serialize,
)]
/// Lifecycle state of shared execution activity.
///
/// # Public members
/// Each variant identifies one activity lifecycle state.
///
/// # Private members
/// None.
pub enum SessionActivityState {
    /// Activity remains in progress.
    Running,
    /// Activity completed successfully.
    Completed,
    /// Activity completed with a failure.
    Failed,
    /// Activity was cancelled.
    Cancelled,
}

/// Shared application-facing name for session activity categories.
pub type AgentActivityKind = SessionActivityKind;

/// Shared application-facing name for session activity lifecycle states.
pub type AgentActivityState = SessionActivityState;

/// Trusted semantic category of a model-visible application tool.
#[derive(
    Clone,
    Copy,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    schemars::JsonSchema,
    serde::Deserialize,
    serde::Serialize,
)]
/// Identifies a semantic or generic tool activity in durable execution history.
pub enum ToolActivityKind {
    /// Reads entities, relations, or semantic projections.
    SemanticRead,
    /// Applies a typed domain mutation through shared change operations.
    SemanticMutation,
    /// Changes a derived projection without changing canonical meaning.
    ProjectionOperation,
    /// Runs validation or another explicitly authorized external process.
    ValidationProcess,
    /// Uses generic interoperability or bootstrap access to opaque content.
    OpaqueFallback,
    /// Creates an isolated child branch for separate work.
    BranchProducing,
}

impl SessionExecutionSelection {
    /// Replaces the future-invocation selection using optimistic concurrency.
    ///
    /// - `self`: Current session selection.
    /// - `expected_revision`: Revision observed by the mutating client.
    /// - `provider_id`: Upstream provider identity, when a model is selected.
    /// - `model_id`: Upstream provider-scoped model identity, when selected.
    /// - `thinking_level`: Supported thinking level, when selected.
    ///
    /// The updated selection with its next mutation revision.
    ///
    /// Returns a conflict for stale revisions or inconsistent absent values.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn replace(
        &mut self,
        expected_revision: u64,
        provider_id: Option<String>,
        model_id: Option<String>,
        thinking_level: Option<String>,
    ) -> Result<(), SelectionError> {
        if expected_revision != self.revision {
            return Err(SelectionError::RevisionConflict {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if provider_id.is_some() != model_id.is_some()
            || (model_id.is_none() && thinking_level.is_some())
        {
            return Err(SelectionError::InconsistentSelection);
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(SelectionError::RevisionExhausted)?;
        self.provider_id = provider_id;
        self.model_id = model_id;
        self.thinking_level = thinking_level;
        Ok(())
    }
}

/// Describes optimistic-concurrency failures for execution selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionError {
    /// The caller's observed revision is stale.
    RevisionConflict { expected: u64, actual: u64 },
    /// Provider and model must be set or cleared together.
    InconsistentSelection,
    /// The selection revision has no representable successor.
    RevisionExhausted,
}

/// Describes how a model result interacted with the derived-result cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CacheDisposition {
    /// No cache lookup was performed.
    NotUsed,
    /// A matching durable result was reused.
    Hit,
    /// A lookup was performed but did not produce a reusable result.
    Miss,
}

/// Records non-secret provenance for one model execution.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelInvocation {
    /// Stable invocation identity.
    pub id: EntityId,
    /// Branch owning the execution.
    pub branch: EntityId,
    /// Explicit activity order within the branch.
    pub ordinal: u64,
    /// Stable agent profile reference.
    pub agent_profile: Option<String>,
    /// Identity of the context supplied to the model.
    pub context_fingerprint: Option<String>,
    /// Safe display identifier for the provider profile.
    pub provider_profile: Option<String>,
    /// Upstream provider identity resolved for this invocation.
    #[serde(default)]
    pub provider_id: Option<String>,
    /// Safe model identifier.
    pub model: Option<String>,
    /// Curated model profile selected for this invocation.
    #[serde(default)]
    pub model_profile: Option<String>,
    /// Provider-native model identifier used by the adapter.
    #[serde(default)]
    pub provider_model_id: Option<String>,
    /// Model-advertised thinking level resolved for this invocation.
    #[serde(default)]
    pub thinking_level: Option<String>,
    /// Fingerprint of the canonical request, never the request secret.
    pub request_fingerprint: Option<String>,
    /// Cache behavior for this invocation.
    pub cache: CacheDisposition,
    /// Result reference when execution completed.
    pub output: Option<EntityId>,
    /// Bounded provider-neutral request captured for later inspection.
    #[serde(default)]
    pub request: Option<ModelRequestProjection>,
}

/// Safe bounded representation of a normalized model request.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelRequestProjection {
    /// Monotonic execution activity order within its branch.
    pub ordinal: u64,
    /// Provider-neutral model identifier selected for the request.
    pub model: String,
    /// Upstream provider identity resolved for this request, when available.
    #[serde(default)]
    pub provider_id: Option<String>,
    /// Safe `ScribeRig` provider profile identifier, when used.
    #[serde(default)]
    pub provider_profile: Option<String>,
    /// Model-advertised thinking level selected for the request.
    pub thinking_level: Option<String>,
    /// Exact safe profile revision used to compile this request.
    #[serde(default)]
    pub profile: Option<ModelContextRevisionRef>,
    /// Active safe component revisions in compilation order.
    #[serde(default)]
    pub active_components: Vec<ModelContextRevisionRef>,
    /// Secret-free identity of compiled `ScribeRig` context.
    #[serde(default)]
    pub context_fingerprint: Option<String>,
    /// Secret-free fingerprint of normalized request inputs, upstream model
    /// configuration, and any validated applied-settings report received.
    ///
    /// The applied report may be partial; this digest identifies captured
    /// provenance and does not prove provider-request equivalence.
    #[serde(default)]
    pub request_fingerprint: Option<String>,
    /// Upstream-defined partial execution configuration snapshot for the model.
    ///
    /// This is historical provenance only. Inspect upstream coverage metadata
    /// before treating it as an execution-equivalence claim.
    #[serde(default)]
    pub model_execution_configuration: Option<serde_json::Value>,
    /// Ordered message sections sent to the provider adapter.
    pub messages: Vec<ModelRequestMessage>,
    /// Tool definitions exposed to the provider adapter.
    pub tools: Vec<ModelRequestTool>,
    /// Safe upstream report of settings applied to the final provider request.
    ///
    /// The opaque JSON snapshot stays upstream-defined; `ScribeRig` stores it
    /// only as historical execution provenance.
    #[serde(default)]
    pub applied_settings: Option<serde_json::Value>,
    /// Versioned digest of the upstream applied-settings report, when received.
    ///
    /// This fingerprints only the attested partial report. It does not prove a
    /// complete effective request because omitted categories remain excluded.
    #[serde(default)]
    pub applied_settings_fingerprint: Option<String>,
    /// Whether any content was omitted to preserve the projection budget.
    pub truncated: bool,
}

/// Safe exact reference to one profile or context-component revision.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelContextRevisionRef {
    /// Stable semantic identity.
    pub identity: String,
    /// Exact immutable revision identity.
    pub revision: String,
}

/// One provider-neutral message retained in a request projection.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelRequestMessage {
    /// Semantic message role.
    pub role: String,
    /// Bounded message content.
    pub content: String,
}

/// One provider-neutral tool schema retained in a request projection.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelRequestTool {
    /// Stable trusted tool name.
    pub name: String,
    /// Bounded tool description.
    pub description: String,
    /// Canonical JSON schema text, when it fits the request budget.
    pub parameters: Option<String>,
}

/// Records complete externally meaningful model output without hidden reasoning.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ModelOutput {
    /// Stable output identity.
    pub id: EntityId,
    /// Invocation that produced this output.
    pub invocation: EntityId,
    /// Complete assistant text when available.
    pub text: Option<String>,
    /// Safe structured-output reference when available.
    pub structured_output: Option<EntityId>,
    /// Completion category such as `stop` or `tool_call`.
    pub finish_reason: Option<String>,
    /// Provider-neutral usage values `(input, output)` when available.
    pub usage: Option<(u64, u64)>,
}

/// Identifies one ordered execution activity in a branch.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ExecutionActivity {
    /// Human input recorded on a branch turn.
    HumanInput(EntityId),
    /// Model invocation activity.
    ModelInvocation(EntityId),
    /// Model output activity.
    ModelOutput(EntityId),
    /// Tool invocation activity.
    ToolInvocation(EntityId),
    /// Tool result activity.
    ToolResult(EntityId),
    /// Child branch start or result reference.
    ChildBranch(EntityId),
    /// Child branch linked to the tool invocation that spawned it.
    ChildBranchSpawned {
        /// Spawned child branch.
        branch: EntityId,
        /// Parent branch tool invocation that requested the child.
        parent_tool_invocation: EntityId,
    },
    /// Durable result committed for the branch.
    BranchResult(EntityId),
    /// Agent output activity.
    AgentOutput(EntityId),
}
