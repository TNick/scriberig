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
    /// Compact terminal result, when one exists.
    pub result: Option<SessionBranchResult>,
}

/// Compact session tree response that avoids provider transcript details.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionReadResponse {
    /// Session identity returned by the server.
    pub session: SessionReference,
    /// Deterministically ordered branch summaries.
    pub branches: Vec<SessionBranch>,
}
