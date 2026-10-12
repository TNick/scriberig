//! Defines provider-neutral model execution records and deterministic activity order.

use scriberig_core::EntityId;

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
    /// Fingerprint of the canonical request, never the request secret.
    pub request_fingerprint: Option<String>,
    /// Cache behavior for this invocation.
    pub cache: CacheDisposition,
    /// Result reference when execution completed.
    pub output: Option<EntityId>,
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
    /// Agent output activity.
    AgentOutput(EntityId),
}
