//! Defines activity values for durable session transport.

use crate::{SessionActivityKind, SessionActivityState, ToolActivityKind};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    /// Owning application operation for model execution activity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
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
    /// Trusted runtime category for a tool invocation.
    #[serde(default)]
    pub tool_category: Option<ToolActivityKind>,
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
