//! Defines public application boundaries for session reads and agent commands.

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

/// Identifies one requested model-profile mutation for a session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionModelSelectionRequest {
    /// Stable session identity to update.
    pub session: EntityId,
    /// Selection revision observed by the client.
    pub expected_revision: u64,
    /// Enabled model-profile identity to select.
    pub model_profile: String,
}

/// Identifies one requested thinking-level mutation for a session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionThinkingSelectionRequest {
    /// Stable session identity to update.
    pub session: EntityId,
    /// Selection revision observed by the client.
    pub expected_revision: u64,
    /// Advertised thinking-level identity to select.
    pub thinking_level: String,
}

/// Holds bounded safe detail for one durable tool result.
///
/// # Public members
/// - result: Stable tool-result identity.
/// - detail: Allowlisted result fields safe for client presentation.
/// - truncated: Whether the detail budget omitted safe values.
///
/// # Private members
/// None.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolResultDetail {
    /// Stable tool-result identity.
    pub result: EntityId,
    /// Whitelisted provider-neutral result fields.
    pub detail: serde_json::Value,
    /// Whether safe fields were truncated to satisfy the response bound.
    pub truncated: bool,
}

/// Owns model-library reads and session-scoped execution selection.
pub trait AgentConsoleBackend {
    /// Lists safe model descriptors and curated profiles.
    ///
    /// # Parameters
    /// - `context`: Authorized application request context.
    ///
    /// # Returns
    /// The configured safe model library.
    ///
    /// # Errors
    /// Returns a permission or configuration failure.
    fn agent_model_library(
        &self,
        context: &crate::ApplicationContext,
    ) -> Result<crate::ModelLibrary, crate::ApplicationError>;

    /// Reads the caller's shared presentation defaults.
    ///
    /// # Parameters
    /// - `_context`: Authorized caller context.
    ///
    /// # Returns
    /// Revisioned shared defaults independent from session history.
    ///
    /// # Errors
    /// Returns unsupported-operation when unavailable.
    fn agent_presentation_preferences(
        &self,
        _context: &crate::ApplicationContext,
    ) -> Result<crate::AgentPresentationPreferenceSnapshot, crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "agent presentation preferences".to_owned(),
        })
    }

    /// Updates shared presentation defaults using optimistic revision checks.
    ///
    /// # Parameters
    /// - `_context`: Authorized caller context.
    /// - `_expected_revision`: Preference revision observed by the caller.
    /// - `_preferences`: Complete independent presentation choices.
    ///
    /// # Returns
    /// The updated authoritative preferences.
    ///
    /// # Errors
    /// Returns unsupported-operation when unavailable.
    fn set_agent_presentation_preferences(
        &self,
        _context: &crate::ApplicationContext,
        _expected_revision: u64,
        _preferences: crate::AgentPresentationPreferences,
    ) -> Result<crate::AgentPresentationPreferenceSnapshot, crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "agent presentation preferences".to_owned(),
        })
    }

    /// Selects a model profile for future session invocations.
    ///
    /// # Parameters
    /// - `context`: Authorized application request context.
    /// - `request`: Session, observed mutation revision, and profile.
    ///
    /// # Returns
    /// The updated shared session selection.
    ///
    /// # Errors
    /// Returns a permission, validation, conflict, or persistence failure.
    fn set_session_model(
        &self,
        context: &crate::ApplicationContext,
        request: &SessionModelSelectionRequest,
    ) -> Result<SessionExecutionSelection, crate::ApplicationError>;

    /// Selects an advertised thinking level for future invocations.
    ///
    /// # Parameters
    /// - `context`: Authorized application request context.
    /// - `request`: Session, observed revision, and thinking level.
    ///
    /// # Returns
    /// The updated shared session selection.
    ///
    /// # Errors
    /// Returns a permission, validation, conflict, or persistence failure.
    fn set_session_thinking_level(
        &self,
        context: &crate::ApplicationContext,
        request: &SessionThinkingSelectionRequest,
    ) -> Result<SessionExecutionSelection, crate::ApplicationError>;

    /// Reads allowlisted detail for one durable tool result.
    ///
    /// # Parameters
    /// - `_context`: Authorized application request context.
    /// - `_session`: Session containing the result.
    /// - `_branch`: Branch containing the result.
    /// - `_result`: Stable tool-result identity.
    ///
    /// # Returns
    /// Bounded safe result detail.
    ///
    /// # Errors
    /// Returns a permission, lookup, or persistence failure.
    fn tool_result_detail(
        &self,
        _context: &crate::ApplicationContext,
        _session: EntityId,
        _branch: EntityId,
        _result: EntityId,
    ) -> Result<ToolResultDetail, crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "tool result detail".to_owned(),
        })
    }

    /// Reads the bounded provider-neutral request captured for an invocation.
    ///
    /// # Parameters
    /// - `_context`: Authorized request context.
    /// - `_session`: Session containing the invocation.
    /// - `_branch`: Branch containing the invocation.
    /// - `_invocation`: Stable model-invocation identity.
    ///
    /// # Returns
    /// The captured safe request projection.
    ///
    /// # Errors
    /// Returns unsupported-operation when unavailable.
    fn model_request_detail(
        &self,
        _context: &crate::ApplicationContext,
        _session: EntityId,
        _branch: EntityId,
        _invocation: EntityId,
    ) -> Result<scriberig_session::ModelRequestProjection, crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "model request detail".to_owned(),
        })
    }
}
