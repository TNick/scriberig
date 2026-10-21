//! Stores model invocations, request projections, and output provenance.

use scriberig_core::EntityId;

use super::{ModelRequestDetailMessage, ModelRequestDetailTool};

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

/// Safe request provenance with bounded summary and on-demand detail.
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
    /// Complete safe message projection for on-demand detail reads.
    ///
    /// This is absent for legacy records. Hidden reasoning and non-text
    /// payloads remain excluded.
    #[serde(default)]
    pub full_messages: Option<Vec<ModelRequestDetailMessage>>,
    /// Tool definitions exposed to the provider adapter.
    pub tools: Vec<ModelRequestTool>,
    /// Complete safe tool projection for on-demand detail reads.
    ///
    /// This is absent for legacy records.
    #[serde(default)]
    pub full_tools: Option<Vec<ModelRequestDetailTool>>,
    /// Whether full detail contains every safe text and tool field.
    ///
    /// False when provider-visible non-text content was omitted or the record
    /// predates complete detail capture.
    #[serde(default)]
    pub content_complete: bool,
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
