//! Defines public application boundaries for session reads and agent commands.

use scriberig_core::{EntityId, OperationId, SnapshotId};
use scriberig_session::Session;

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
    /// User-authored command text.
    pub prompt: String,
    /// Exact semantic scope supplied to the agent runtime.
    pub selection: Option<AgentSelection>,
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

/// Owns session and agent operations at the application boundary.
pub trait SessionBackend {
    /// Reads one public session snapshot.
    ///
    /// # Parameters
    /// - `context`: Authorized caller context.
    /// - `request`: Stable session request.
    ///
    /// # Returns
    /// The session state supplied by the owning subsystem.
    ///
    /// # Errors
    /// Returns a structured application failure.
    fn read_session(
        &self,
        context: &crate::ApplicationContext,
        request: &SessionRequest,
    ) -> Result<Session, crate::ApplicationError>;

    /// Starts one agent command without making its output canonical.
    ///
    /// # Parameters
    /// - `context`: Authorized caller context.
    /// - `request`: Session, branch, and prompt input.
    ///
    /// # Returns
    /// Stable operation identity for lifecycle observation.
    ///
    /// # Errors
    /// Returns a structured application failure.
    fn submit_agent_command(
        &self,
        context: &crate::ApplicationContext,
        request: &AgentCommandRequest,
    ) -> Result<OperationId, crate::ApplicationError>;
}
