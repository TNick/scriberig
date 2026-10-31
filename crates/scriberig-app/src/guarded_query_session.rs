//! Applies session authorization through guarded read providers.

use super::{ApplicationContext, ApplicationError, GuardedQueryService};

impl<S: crate::SessionBackend> crate::SessionBackend for GuardedQueryService<S> {
    /// Delegates exact continuation through the wrapped application boundary.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Exact continuation location and user input.
    ///
    /// Authoritative operation, session, and branch handles.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn continue_agent_at(
        &self,
        context: &ApplicationContext,
        request: crate::AgentContinueAtRequest,
    ) -> Result<crate::AgentContinuationAccepted, ApplicationError> {
        self.inner.continue_agent_at(context, request)
    }

    /// Delegates new-session creation through the wrapped application boundary.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Initial user input and optional semantic selection.
    ///
    /// Authoritative operation, session, and root branch handles.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn start_new_session_agent(
        &self,
        context: &ApplicationContext,
        request: crate::NewSessionAgentRequest,
    ) -> Result<crate::NewSessionAgentStart, ApplicationError> {
        self.inner.start_new_session_agent(context, request)
    }

    /// Delegates a session read through the wrapped application boundary.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Stable session identity to read.
    ///
    /// The requested session snapshot.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        context: &ApplicationContext,
        request: &crate::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        self.inner.read_session(context, request)
    }

    /// Delegates compact session projection reads through the guarded service.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Stable session identity to read.
    ///
    /// The wrapped session projection.
    ///
    /// # Returns
    /// The compact projection when supported by the wrapped service.
    ///
    /// # Errors
    /// Returns the wrapped session backend error.
    fn read_session_projection(
        &self,
        context: &ApplicationContext,
        request: &crate::SessionRequest,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        self.inner.read_session_projection(context, request)
    }

    /// Delegates durable plan creation through the guarded service.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Goal, exact anchor, nodes, and dependencies.
    ///
    /// The authoritative persisted plan.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_plan(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanCreateRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.inner.create_plan(context, request)
    }

    /// Delegates a validated plan revision through the guarded service.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Complete plan revision proposal.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn replan(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanReplanRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.inner.replan(context, request)
    }

    /// Delegates an optimistic plan lifecycle operation.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Plan identity and expected revision.
    /// - `action`: Requested lifecycle transition.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn plan_lifecycle(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanLifecycleRequest,
        action: crate::PlanLifecycleAction,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.inner.plan_lifecycle(context, request, action)
    }

    /// Delegates a safe node assignment mutation.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Plan, node, revision, and assignment.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set_plan_node_assignment(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanAssignmentRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.inner.set_plan_node_assignment(context, request)
    }

    /// Delegates an agent command through the wrapped application boundary.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Session command to submit.
    ///
    /// The accepted operation identity.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit_agent_command(
        &self,
        context: &ApplicationContext,
        request: &crate::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        self.inner.submit_agent_command(context, request)
    }
}
