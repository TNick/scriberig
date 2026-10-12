//! Defines provider-neutral agent operation protocol values.

use crate::OperationStatusResponse;
use crate::{EntityReference, SnapshotReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Identifies a session in an agent request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionReference {
    /// Stable session identity.
    pub id: String,
}

/// Identifies a branch in an agent continuation request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct BranchReference {
    /// Stable branch identity.
    pub id: String,
}

/// Identifies the exact semantic scope supplied to an agent.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AgentSelection {
    /// Selected semantic entity.
    pub entity: EntityReference,
    /// Snapshot containing the selected entity.
    pub snapshot: SnapshotReference,
}

/// Identifies a candidate change that must use normal migration review.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct CandidateChangeReference {
    /// Migration proposal to review, when one exists.
    #[serde(default)]
    pub migration: Option<String>,
    /// Semantic entity affected by the candidate.
    #[serde(default)]
    pub entity: Option<EntityReference>,
    /// Candidate snapshot produced by the proposal, when known.
    #[serde(default)]
    pub candidate_snapshot: Option<SnapshotReference>,
    /// Safe validation evidence handles associated with the candidate.
    #[serde(default)]
    pub validation: Vec<String>,
}

/// Provider-neutral durable terminal result for one agent operation.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AgentResult {
    /// Session containing the completed work.
    pub session: SessionReference,
    /// Branch receiving the completed work.
    pub root_branch: BranchReference,
    /// Provider-neutral final output.
    pub final_output: String,
    /// Durable branch-result identity.
    pub branch_result: String,
    /// Semantic entities changed by the operation.
    pub produced_changes: Vec<crate::EntityReference>,
    /// Validation summaries associated with the result.
    pub validation_results: Vec<String>,
    /// Questions left unresolved by the operation.
    pub unresolved_questions: Vec<String>,
    /// Candidate changes that require normal review and validation.
    #[serde(default)]
    pub candidate_changes: Vec<CandidateChangeReference>,
}

/// Starts an agent operation at a session root.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AgentStartRequest {
    /// Session receiving the new root operation.
    pub session: SessionReference,
    /// User-authored instruction.
    pub prompt: String,
    /// Exact semantic scope for the requested work.
    #[serde(default)]
    pub selection: Option<AgentSelection>,
}

/// Continues an agent operation on an existing session branch.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AgentContinueRequest {
    /// Session containing the target branch.
    pub session: SessionReference,
    /// Existing branch receiving the follow-up.
    pub branch: BranchReference,
    /// User-authored follow-up instruction.
    pub prompt: String,
    /// Exact semantic scope for the requested work.
    #[serde(default)]
    pub selection: Option<AgentSelection>,
}

/// Returns the shared operation handle accepted for agent execution.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AgentAcceptedResponse {
    /// Shared application operation status.
    pub operation: OperationStatusResponse,
    /// Session receiving the operation.
    pub session: SessionReference,
    /// Branch targeted by continuation, when present.
    pub branch: Option<BranchReference>,
}
