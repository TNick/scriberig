//! Defines the shared session and plan mutation boundary.

use super::{
    AgentCommandRequest, AgentContinuationAccepted, AgentContinueAtRequest, ApplicationContext,
    ApplicationError, EntityId, NewSessionAgentRequest, NewSessionAgentStart, OperationId,
    PlanAssignmentRequest, PlanCreateRequest, PlanLifecycleAction, PlanLifecycleRequest,
    PlanReplanRequest, Session, SessionRequest,
};

/// Owns session and agent operations at the application boundary.
pub trait SessionBackend {
    /// Starts a new session through the authoritative application boundary.
    ///
    /// - `_context`: Authorized application request context.
    /// - `_request`: Initial human input and optional semantic selection.
    ///
    /// Application allocated operation, session, and root branch handles.
    ///
    /// Returns unsupported-operation when new-session execution is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn start_new_session_agent(
        &self,
        _context: &ApplicationContext,
        _request: NewSessionAgentRequest,
    ) -> Result<NewSessionAgentStart, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "new session agent start".to_owned(),
        })
    }

    /// Continues an existing session at an exact validated reply anchor.
    ///
    /// - `_context`: Authorized application request context.
    /// - `_request`: Session, exact anchor, and user instruction.
    ///
    /// Authoritative operation, session, and continuation branch handles.
    ///
    /// Returns unsupported-operation when exact continuation is unavailable.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn continue_agent_at(
        &self,
        _context: &ApplicationContext,
        _request: AgentContinueAtRequest,
    ) -> Result<AgentContinuationAccepted, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "exact agent continuation".to_owned(),
        })
    }

    /// Reads one public session snapshot.
    ///
    /// - `context`: Authorized caller context.
    /// - `request`: Stable session request.
    ///
    /// The session state supplied by the owning subsystem.
    ///
    /// Returns a structured application failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        context: &crate::ApplicationContext,
        request: &SessionRequest,
    ) -> Result<Session, crate::ApplicationError>;

    /// Persists a complete authorized session snapshot.
    ///
    /// - `context`: Authorized caller context.
    /// - `session`: Complete changed session value.
    ///
    /// Returns a structured application failure.
    /// # Parameters
    /// - `_context`: Input for this operation.
    /// - `_session`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn save_session(
        &self,
        _context: &crate::ApplicationContext,
        _session: &Session,
    ) -> Result<(), crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "session mutation".to_owned(),
        })
    }

    /// Atomically replaces one plan while preserving other session changes.
    /// Newly added branch records are merged and linked to their parent in the
    /// same operation; existing branches are never overwritten.
    ///
    /// - `context`: Authorized caller context.
    /// - `session`: Snapshot containing the proposed plan.
    /// - `plan`: Stable plan identity.
    /// - `expected_revision`: Plan revision observed by the caller.
    ///
    /// Returns conflict, persistence, or unsupported-operation failures.
    /// # Parameters
    /// - `_context`: Input for this operation.
    /// - `_session`: Input for this operation.
    /// - `_plan`: Input for this operation.
    /// - `_expected_revision`: Input for this operation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_swap_plan(
        &self,
        _context: &crate::ApplicationContext,
        _session: &Session,
        _plan: EntityId,
        _expected_revision: u64,
    ) -> Result<(), crate::ApplicationError> {
        Err(crate::ApplicationError::UnsupportedOperation {
            operation: "atomic plan update".to_owned(),
        })
    }

    /// Creates a durable structured plan through application validation.
    ///
    /// - `_context`: Authorized caller context.
    /// - `_request`: Goal, exact anchor, nodes, and dependencies.
    ///
    /// The authoritative persisted plan.
    ///
    /// Returns unsupported-operation or structured validation/persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_plan(
        &self,
        _context: &ApplicationContext,
        _request: &PlanCreateRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "plan creation".to_owned(),
        })
    }

    /// Applies a complete validated revision to one durable plan.
    ///
    /// - `_context`: Authorized caller context.
    /// - `_request`: Plan revision proposal and observed revision.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns unsupported-operation or structured plan errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn replan(
        &self,
        _context: &ApplicationContext,
        _request: &PlanReplanRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "plan replan".to_owned(),
        })
    }

    /// Applies one shared optimistic plan lifecycle operation.
    ///
    /// - `_context`: Authorized caller context.
    /// - `_request`: Plan and observed revision.
    /// - `_action`: Requested lifecycle transition.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns unsupported-operation, conflict, validation, or persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn plan_lifecycle(
        &self,
        _context: &ApplicationContext,
        _request: &PlanLifecycleRequest,
        _action: PlanLifecycleAction,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "plan lifecycle".to_owned(),
        })
    }

    /// Changes one plan node's credential-safe model-library assignment.
    ///
    /// - `_context`: Authorized caller context.
    /// - `_request`: Plan, node, observed revision, and new assignment.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns unsupported-operation or structured validation/persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_plan_node_assignment(
        &self,
        _context: &ApplicationContext,
        _request: &PlanAssignmentRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "plan node assignment".to_owned(),
        })
    }

    /// Starts one agent command without making its output canonical.
    ///
    /// - `context`: Authorized caller context.
    /// - `request`: Session, branch, and prompt input.
    ///
    /// Stable operation identity for lifecycle observation.
    ///
    /// Returns a structured application failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit_agent_command(
        &self,
        context: &crate::ApplicationContext,
        request: &AgentCommandRequest,
    ) -> Result<OperationId, crate::ApplicationError>;
}
