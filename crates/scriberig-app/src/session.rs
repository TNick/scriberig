//! Defines public application boundaries for session reads and agent commands.

use crate::{ApplicationContext, ApplicationError};
use scriberig_core::{EntityId, OperationId, SnapshotId};
use scriberig_session::{Session, SessionExecutionSelection};

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

/// Describes one proposed node before application identities are allocated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanNodeDraft {
    /// Stable request-local key used by dependency declarations.
    pub key: String,
    /// Optional parent draft index; absent nodes attach to the anchor branch.
    pub parent_index: Option<usize>,
    /// Human-readable work title.
    pub title: String,
    /// Focused work instruction.
    pub purpose: String,
    /// Expected provider-neutral result contract.
    pub expected_result: Option<scriberig_session::ExpectedResult>,
    /// Semantic handles selected for this work.
    pub semantic_scope: Vec<String>,
    /// Optional safe model-library assignment.
    pub assignment: Option<scriberig_session::ModelAssignment>,
}

/// Requests durable plan creation at an exact session anchor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanCreateRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Exact conversation location where the plan is created.
    pub anchor: scriberig_session::ContinuationAnchor,
    /// Durable plan goal.
    pub goal: String,
    /// Structured untrusted proposal nodes.
    pub nodes: Vec<PlanNodeDraft>,
    /// Dependency index pairs `(prerequisite, dependent)`.
    pub dependencies: Vec<(usize, usize)>,
}

/// Requests one optimistic plan lifecycle mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanLifecycleRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
}

/// Selects a durable lifecycle transition for a plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlanLifecycleAction {
    /// Enables auto-advancing coordination.
    Start,
    /// Pauses dispatch while preserving plan state.
    Pause,
    /// Resumes dispatch for a paused plan.
    Resume,
    /// Stops auto-advancement.
    Stop,
    /// Completes only after explicit durable node results are present.
    Complete,
}

/// Requests a safe model assignment change for one plan node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanAssignmentRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Stable plan-node identity.
    pub node: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
    /// New library-validated assignment, or `None` to inherit defaults.
    pub assignment: Option<scriberig_session::ModelAssignment>,
}

/// Identifies an existing or newly proposed node in a replan request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanNodeReference {
    /// Existing durable node identity.
    Existing(EntityId),
    /// Request-local key for a proposed node.
    Proposed(String),
}

/// Requests one optimistic durable plan revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanReplanRequest {
    /// Owning session identity.
    pub session: EntityId,
    /// Stable plan identity.
    pub plan: EntityId,
    /// Plan revision observed by the caller.
    pub expected_revision: u64,
    /// New structured nodes to append.
    pub additions: Vec<PlanNodeDraft>,
    /// Explicit dependency references `(prerequisite, dependent)`.
    pub dependencies: Vec<(PlanNodeReference, PlanNodeReference)>,
    /// Existing dependency edges to remove explicitly.
    pub removed_dependencies: Vec<(EntityId, EntityId)>,
    /// Existing node and proposed replacement key pairs.
    pub superseded: Vec<(EntityId, String)>,
    /// Safe assignment changes for existing unstarted nodes.
    pub assignment_changes: Vec<(EntityId, Option<scriberig_session::ModelAssignment>)>,
    /// Completed terminal node identities that the proposal preserves.
    pub preserved_terminal_nodes: Vec<EntityId>,
    /// Safe explanation or trigger recorded in revision history.
    pub reason: String,
}

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
    AgentConsoleBackend, SessionModelSelectionRequest, SessionThinkingSelectionRequest,
    ToolResultDetail,
};
