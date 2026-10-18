//! Defines compact provider-neutral session observation protocol values.

use crate::SessionReference;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Requests one durable session tree.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionReadRequest {
    /// Session identity to inspect.
    pub session: SessionReference,
    /// Optional branch whose activity should be loaded on demand.
    #[serde(default)]
    pub branch: Option<crate::BranchReference>,
}

/// Requests one stored model request projection by durable invocation identity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelRequestDetailRequest {
    /// Session containing the invocation.
    pub session: SessionReference,
    /// Branch containing the invocation.
    pub branch: crate::BranchReference,
    /// Stable model invocation activity identity.
    pub invocation: String,
}

/// One safe message in a stored provider-neutral model request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelRequestMessageDetail {
    /// Provider-neutral message role.
    pub role: String,
    /// Captured message content.
    pub content: String,
}

/// One trusted tool definition in a stored model request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelRequestToolDetail {
    /// Stable provider-facing tool name.
    pub name: String,
    /// Safe descriptive text.
    pub description: String,
    /// Bounded JSON Schema text when it fit in the capture budget.
    pub parameters: Option<String>,
}

/// Safe provider-neutral detail captured for one model invocation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ModelRequestDetailResponse {
    /// Stable model invocation identity.
    pub invocation: String,
    /// Normalized model identifier.
    pub model: String,
    /// Selected provider-neutral thinking level, when configured.
    pub thinking_level: Option<String>,
    /// Ordered normalized messages.
    pub messages: Vec<ModelRequestMessageDetail>,
    /// Trusted tools exposed to the model.
    pub tools: Vec<ModelRequestToolDetail>,
    /// Whether content was omitted to respect capture bounds.
    pub truncated: bool,
}

/// Requests expanded bounded detail for one durable tool result.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ToolResultDetailRequest {
    /// Session containing the result.
    pub session: SessionReference,
    /// Branch containing the result.
    pub branch: crate::BranchReference,
    /// Stable tool result identity.
    pub result: String,
}

/// Safe bounded structured detail for one durable tool result.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ToolResultDetailResponse {
    /// Stable tool result identity.
    pub result: String,
    /// Whitelisted provider-neutral result fields.
    pub detail: serde_json::Value,
    /// Whether safe fields were truncated to satisfy the response bound.
    pub truncated: bool,
}

/// Durable branch lifecycle visible to clients.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionBranchStatus {
    /// Branch is still accepting activity.
    Active,
    /// Branch reached a terminal result.
    Completed,
    /// Branch was abandoned.
    Abandoned,
}

/// One ordered branch activity summary.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionTurn {
    /// Stable turn identity.
    pub id: String,
    /// Explicit activity order within the branch.
    pub ordinal: u64,
    /// Whether human input exists for this turn.
    pub has_human_input: bool,
    /// Whether agent output exists for this turn.
    pub has_agent_output: bool,
    /// Durable provider-neutral agent output, when committed.
    #[serde(default)]
    pub agent_output: Option<String>,
    /// Optional tool invocation identity.
    pub tool_invocation: Option<String>,
    /// Optional tool result identity.
    pub tool_result: Option<String>,
}

/// Provider-neutral category for a durable causal activity record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionActivityKind {
    /// Human supplied a prompt or instruction.
    HumanInput,
    /// Model request was issued.
    ModelInvocation,
    /// Externally meaningful model output was committed.
    AssistantOutput,
    /// A trusted tool was invoked.
    ToolInvocation,
    /// A tool returned or failed.
    ToolResult,
    /// A child branch was created.
    ChildBranch,
    /// A durable branch result was committed.
    BranchResult,
}

/// Lifecycle state of a durable activity item.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum SessionActivityState {
    /// Activity has completed successfully.
    Completed,
    /// Activity remains in progress.
    Running,
    /// Activity completed with a failure.
    Failed,
}

/// Safe projection of one durable execution activity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionActivityItem {
    /// Stable activity identity.
    pub id: String,
    /// Session containing the activity.
    pub session: String,
    /// Order within the owning branch.
    pub sequence: u64,
    /// Branch containing this activity.
    pub branch: String,
    /// Causal parent activity, when one is known.
    #[serde(default)]
    pub parent: Option<String>,
    /// Provider-neutral activity category.
    pub kind: SessionActivityKind,
    /// Durable lifecycle state.
    pub state: SessionActivityState,
    /// Safe concise display text.
    pub summary: String,
    /// Stable handle for loading a bounded detail view when available.
    #[serde(default)]
    pub durable_ref: Option<String>,
    /// Safe model identity for a model invocation.
    #[serde(default)]
    pub model: Option<String>,
    /// Safe thinking-level identity for a model invocation.
    #[serde(default)]
    pub thinking_level: Option<String>,
    /// Trusted tool name for a tool activity.
    #[serde(default)]
    pub tool_name: Option<String>,
    /// Schema-redacted structured arguments for detailed tool presentation.
    #[serde(default)]
    pub tool_arguments: Option<serde_json::Value>,
    /// Bounded safe structured fields for full tool-result presentation.
    #[serde(default)]
    pub tool_result_preview: Option<serde_json::Value>,
}

/// Compact durable result reference for a branch.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionBranchResult {
    /// Stable result identity.
    pub id: String,
    /// Semantic handles produced by the branch.
    pub references: Vec<String>,
    /// Stable non-entity result handles produced by the branch.
    #[serde(default)]
    pub result_handles: Vec<String>,
}

/// Compact branch tree node returned by session reads.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionBranch {
    /// Stable branch identity.
    pub id: String,
    /// Explicit parent branch identity, when nested.
    #[serde(default)]
    pub parent: Option<String>,
    /// Explicit child branch identities.
    #[serde(default)]
    pub children: Vec<String>,
    /// Human-facing branch title.
    pub title: String,
    /// Durable branch lifecycle.
    pub status: SessionBranchStatus,
    /// Explicitly ordered activity summaries.
    pub turns: Vec<SessionTurn>,
    /// Ordered durable execution activity without raw tool payloads.
    #[serde(default)]
    pub activity: Vec<SessionActivityItem>,
    /// Compact terminal result, when one exists.
    pub result: Option<SessionBranchResult>,
}

/// Session-scoped model selection used by future agent invocations.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionExecutionSelection {
    /// Mutation revision for optimistic concurrency.
    pub revision: u64,
    /// Curated model profile identity, when selected.
    #[serde(default)]
    pub model_profile: Option<String>,
    /// Provider-neutral model identity, when selected.
    #[serde(default)]
    pub model: Option<String>,
    /// Model-advertised thinking level, when selected.
    #[serde(default)]
    pub thinking_level: Option<String>,
}

/// Compact session tree response that avoids provider transcript details.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionReadResponse {
    /// Session identity returned by the server.
    pub session: SessionReference,
    /// Deterministically ordered branch summaries.
    pub branches: Vec<SessionBranch>,
    /// Current session selection for future invocations.
    #[serde(default)]
    pub execution: SessionExecutionSelection,
}
