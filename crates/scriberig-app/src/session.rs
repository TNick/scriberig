//! Defines public application boundaries for session reads and agent commands.

use crate::{ApplicationContext, ApplicationError};
use scriberig_core::{EntityId, OperationId, SnapshotId};
use scriberig_session::{Session, SessionExecutionSelection};

#[path = "session/plan_requests.rs"]
mod plan_requests;

#[path = "session/plan_mutations.rs"]
mod plan_mutations;
pub use plan_mutations::{
    PlanChildDraft, PlanDecomposeRequest, PlanNextStepRequest, PlanSemanticMutation,
    PlanSemanticMutationRequest,
};

/// Describes the durable terminal result of one agent operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentResult {
    /// Session containing the completed work.
    pub session: EntityId,
    /// Branch receiving the completed work.
    pub root_branch: EntityId,
    /// Provider-neutral final output text.
    pub final_output: String,
    /// Durable branch-result identity.
    pub branch_result: EntityId,
    /// Semantic changes produced by the operation.
    pub produced_changes: Vec<EntityId>,
    /// Validation summaries associated with the result.
    pub validation_results: Vec<String>,
    /// Questions left unresolved by the operation.
    pub unresolved_questions: Vec<String>,
    /// Candidate changes that require normal review and validation.
    pub candidate_changes: Vec<CandidateChange>,
}

/// Identifies a candidate migration without accepting it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateChange {
    /// Migration proposal handle, when available.
    pub migration: Option<String>,
    /// Entity affected by the candidate.
    pub entity: Option<EntityId>,
    /// Candidate snapshot produced by the proposal, when available.
    pub candidate_snapshot: Option<SnapshotId>,
    /// Validation evidence handles.
    pub validation: Vec<String>,
}

/// Carries one executed tool call across the shared runtime/application boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolExecutionEvidence {
    /// Monotonic execution activity order within the branch.
    pub ordinal: u64,
    /// Stable provider-facing tool name.
    pub name: String,
    /// Exact JSON arguments submitted to the application tool.
    pub input: Vec<u8>,
    /// Schema-redacted JSON arguments safe for client presentation.
    pub presentation_input: Option<Vec<u8>>,
    /// Optional trusted compact invocation label safe for client presentation.
    pub presentation_label: Option<String>,
    /// Trusted semantic category persisted with the invocation when known.
    pub category: Option<scriberig_session::ToolActivityKind>,
    /// Structured bounded tool result encoded as JSON.
    pub output: Vec<u8>,
    /// Whether the tool returned a structured failure.
    pub failed: bool,
    /// Durable child branch produced by this delegation tool, when applicable.
    pub child_branch: Option<scriberig_core::EntityId>,
}

/// Identifies a session read request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SessionRequest {
    /// Stable session entity identity.
    pub session: EntityId,
}

/// Identifies an agent command submitted to a session branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentCommandRequest {
    /// Target session entity.
    pub session: EntityId,
    /// Optional branch receiving the new turn.
    pub branch: Option<EntityId>,
    /// Exact continuation location for historical replies.
    pub anchor: Option<scriberig_session::ContinuationAnchor>,
    /// User-authored command text.
    pub prompt: String,
    /// Exact semantic scope supplied to the agent runtime.
    pub selection: Option<AgentSelection>,
}

/// Requests a new session start with optional exact semantic scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewSessionAgentRequest {
    /// User-authored first instruction.
    pub prompt: String,
    /// Optional exact semantic selection.
    pub selection: Option<AgentSelection>,
}

/// Requests an exact-anchor continuation with optional semantic scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentContinueAtRequest {
    /// Existing session identity.
    pub session: EntityId,
    /// Exact durable continuation location.
    pub anchor: scriberig_session::ContinuationAnchor,
    /// User-authored follow-up instruction.
    pub prompt: String,
    /// Optional exact semantic selection.
    pub selection: Option<AgentSelection>,
}

/// Returns the authoritative handles allocated for a new agent session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewSessionAgentStart {
    /// Shared operation identity.
    pub operation: OperationId,
    /// Newly created session identity.
    pub session: EntityId,
    /// Root branch receiving the first user input.
    pub root_branch: EntityId,
}

/// Returns handles accepted for an exact historical continuation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentContinuationAccepted {
    /// Shared operation identity.
    pub operation: OperationId,
    /// Existing session identity.
    pub session: EntityId,
    /// New branch created at the exact continuation anchor.
    pub branch: EntityId,
}

pub use plan_requests::*;

/// Identifies the exact semantic scope supplied to an agent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSelection {
    /// Semantic project containing the selected entity.
    pub project: Option<scriberig_core::ProjectId>,
    /// Semantic project root at the exact snapshot.
    pub root: EntityId,
    /// Selected semantic entity.
    pub entity: EntityId,
    /// Snapshot containing the selected entity.
    pub snapshot: SnapshotId,
}

/// Identifies a locally accepted agent operation before execution completes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAgentHandle {
    /// Shared operation identity.
    pub operation: OperationId,
    /// Session receiving the execution.
    pub session: EntityId,
    /// Root branch receiving the execution.
    pub root_branch: EntityId,
    /// Human instruction recorded on the root turn.
    pub prompt: String,
    /// Exact semantic scope selected when this operation was accepted.
    pub selection: Option<AgentSelection>,
}

/// Identifies a locally accepted nested agent operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalChildAgentHandle {
    /// Shared operation identity.
    pub operation: OperationId,
    /// Session receiving the child execution.
    pub session: EntityId,
    /// Parent branch owning the delegation.
    pub parent_branch: EntityId,
    /// Child branch receiving the execution.
    pub child_branch: EntityId,
    /// Child purpose recorded in the session graph.
    pub purpose: String,
    /// Human instruction recorded on the child turn.
    pub prompt: String,
}

#[path = "session/backend.rs"]
mod backend;
pub use backend::SessionBackend;
#[path = "session/agent_console.rs"]
mod agent_console;
pub use agent_console::{
    AgentConsoleBackend, LegacyExecutionMappingRequest, LegacyPlanAssignmentMappingRequest,
    SessionModelSelectionRequest, SessionThinkingSelectionRequest, ToolResultDetail,
};
