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

    /// Delegates paged node attempt history through the wrapped session boundary.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Stable node identity, exact snapshot, and page range.
    ///
    /// The requested attempt page.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns the wrapped session backend error.
    fn read_plan_attempt_history(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanAttemptHistoryRequest,
    ) -> Result<Vec<scriberig_session::PlanSemanticAttempt>, ApplicationError> {
        self.inner.read_plan_attempt_history(context, request)
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

    /// Delegates a failed-node retry through the wrapped application boundary.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Session, plan, node, and observed revision.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn retry_plan_node(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanRetryRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.inner.retry_plan_node(context, request)
    }

    /// Delegates a next-step mutation through the wrapped application boundary.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Plan revision and proposed next-step fields.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn add_plan_next_step(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanNextStepRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.inner.add_plan_next_step(context, request)
    }

    /// Delegates a decomposition mutation through the wrapped boundary.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Parent node, expected revision, and proposed children.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns the wrapped session backend error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn decompose_plan_node(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanDecomposeRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.inner.decompose_plan_node(context, request)
    }

    /// Delegates one typed semantic plan mutation through the wrapped boundary.
    ///
    /// - `context`: Caller authorization and snapshot context.
    /// - `request`: Exact plan revision and mutation details.
    ///
    /// Updated plan.
    /// # Returns
    /// The updated plan.
    /// # Errors
    /// Returns the wrapped session backend error.
    fn mutate_plan_semantics(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanSemanticMutationRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        self.inner.mutate_plan_semantics(context, request)
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
