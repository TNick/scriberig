//! Defines provider-neutral model execution records and deterministic activity order.

use scriberig_core::EntityId;

/// Session-scoped model configuration for future invocations.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct SessionExecutionSelection {
    /// Monotonic mutation revision, independent from event cursors.
    pub revision: u64,
    /// Selected curated model-profile identity.
    pub model_profile: Option<String>,
    /// Selected provider-neutral model identity.
    pub model: Option<String>,
    /// Selected model-advertised thinking-level identity.
    pub thinking_level: Option<String>,
}

impl SessionExecutionSelection {
    /// Replaces the future-invocation selection using optimistic concurrency.
    ///
    /// # Parameters
    /// - `self`: Current session selection.
    /// - `expected_revision`: Revision observed by the mutating client.
    /// - `model_profile`: Profile identity, when a model is selected.
    /// - `model`: Provider-neutral model identity, when selected.
    /// - `thinking_level`: Supported thinking level, when selected.
    ///
    /// # Returns
    /// The updated selection with its next mutation revision.
    ///
    /// # Errors
    /// Returns a conflict for stale revisions or inconsistent absent values.
    pub fn replace(
        &mut self,
        expected_revision: u64,
        model_profile: Option<String>,
        model: Option<String>,
        thinking_level: Option<String>,
    ) -> Result<(), SelectionError> {
        if expected_revision != self.revision {
            return Err(SelectionError::RevisionConflict {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if model_profile.is_some() != model.is_some()
            || (model.is_none() && thinking_level.is_some())
        {
            return Err(SelectionError::InconsistentSelection);
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or(SelectionError::RevisionExhausted)?;
        self.model_profile = model_profile;
        self.model = model;
        self.thinking_level = thinking_level;
        Ok(())
    }
}

/// Describes optimistic-concurrency failures for execution selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionError {
    /// The caller's observed revision is stale.
    RevisionConflict { expected: u64, actual: u64 },
    /// Model and profile must be set or cleared together.
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
    /// Model-advertised thinking level selected for the request.
    pub thinking_level: Option<String>,
    /// Ordered message sections sent to the provider adapter.
    pub messages: Vec<ModelRequestMessage>,
    /// Tool definitions exposed to the provider adapter.
    pub tools: Vec<ModelRequestTool>,
    /// Whether any content was omitted to preserve the projection budget.
    pub truncated: bool,
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
