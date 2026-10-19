//! Defines ordered, provider-neutral turns and tool interactions.
use scriberig_core::EntityId;
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct HumanInput {
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct AgentOutput {
    pub text: String,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct ToolInvocation {
    pub id: EntityId,
    pub name: String,
    pub input: Vec<u8>,
    /// Model invocation whose response requested this tool, when known.
    #[serde(default)]
    pub parent_model_invocation: Option<EntityId>,
    /// Optional schema-redacted structured arguments for safe presentation.
    #[serde(default)]
    pub presentation_input: Option<Vec<u8>>,
    /// Optional trusted compact command label for presentation.
    #[serde(default)]
    pub presentation_label: Option<String>,
    /// Trusted semantic category used by clients without parsing the tool name.
    #[serde(default)]
    pub category: Option<crate::ToolActivityKind>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct ToolResult {
    pub id: EntityId,
    pub invocation: EntityId,
    pub output: Vec<u8>,
    pub failed: bool,
    /// Optional bounded, allowlisted result preview for presentation.
    #[serde(default)]
    pub presentation_preview: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
/// Session-domain structured value.
pub struct Turn {
    pub id: EntityId,
    pub ordinal: u64,
    pub human: Option<HumanInput>,
    pub agent: Option<AgentOutput>,
    pub tool_invocation: Option<EntityId>,
    pub tool_result: Option<EntityId>,
}

/// Identifies an exact durable reply location in a session branch.
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Deserialize, serde::Serialize,
)]
/// Identifies an exact durable location from which a user may continue.
pub enum ContinuationAnchor {
    /// Continue from the current durable end of a branch.
    BranchHead { branch: EntityId },
    /// Continue after a turn containing an externally visible assistant output.
    AssistantOutput { branch: EntityId, turn: EntityId },
    /// Continue after a user input, whether or not the turn has an answer.
    HumanInput { branch: EntityId, turn: EntityId },
    /// Continue from a committed branch result.
    BranchResult { branch: EntityId, result: EntityId },
}

/// Resolved location of a continuation anchor in the immutable session snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedContinuationAnchor {
    /// Branch containing the anchor.
    pub branch: EntityId,
    /// Number of turns included by the anchor.
    pub turn_count: usize,
    /// Result identity when the anchor is a branch result.
    pub result: Option<EntityId>,
}

/// Records why and where a continuation branch was created.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ContinuationProvenance {
    /// Exact durable point used as the branch origin.
    pub anchor: ContinuationAnchor,
    /// Product/runtime cause for creating the branch.
    pub origin: ContinuationOrigin,
}

/// Classifies continuation branch creation without changing branch semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum ContinuationOrigin {
    /// A user replied to an existing conversation point.
    HumanContinuation,
    /// An agent delegated focused work.
    AgentDelegation,
    /// A plan node began execution.
    PlanExecution,
    /// A separate attempt was requested.
    Retry,
    /// Validation work was split into its own branch.
    Validation,
    /// An alternative approach was requested.
    Alternative,
    /// Runtime policy created an isolated branch.
    RuntimePolicy,
}
