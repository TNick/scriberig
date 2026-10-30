//! Provides semantic client methods for durable session detail.

use super::Client;
use crate::error::ClientError;
use scriberig_protocol::{
    AgentPresentationPreferencesReadRequest, AgentPresentationPreferencesResponse,
    AgentPresentationPreferencesSetRequest, BranchReference, ModelRequestDetailRequest,
    ModelRequestDetailResponse, ProtocolTransport, PublicAgentPresentationPreferences, Request,
    Response, SessionPlan, SessionPlanAssignmentRequest, SessionPlanAttemptHistoryPage,
    SessionPlanAttemptHistoryRequest, SessionPlanCreateRequest, SessionPlanDecomposeRequest,
    SessionPlanLifecycleRequest, SessionPlanNextStepRequest, SessionPlanReplanRequest,
    SessionPlanRetryRequest, SessionPlanSemanticMutationRequest, SessionReadRequest,
    SessionReadResponse, SessionReference, ToolResultDetailRequest, ToolResultDetailResponse,
};

impl<T: ProtocolTransport> Client<T> {
    /// Creates a durable plan from a structured, provider-neutral proposal.
    ///
    /// - `request`: Exact anchor and validated plan proposal input.
    ///
    /// The authoritative created plan.
    ///
    /// Returns a structured server or transport error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn create_plan(
        &self,
        request: SessionPlanCreateRequest,
    ) -> Result<SessionPlan, ClientError> {
        match self.send(Request::PlanCreate(request))?.response {
            Response::Plan(plan) => Ok(plan),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Applies one optimistic plan lifecycle transition.
    ///
    /// - `request`: Plan, observed revision, and requested action.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns a structured server or transport error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn plan_lifecycle(
        &self,
        request: SessionPlanLifecycleRequest,
    ) -> Result<SessionPlan, ClientError> {
        match self.send(Request::PlanLifecycle(request))?.response {
            Response::Plan(plan) => Ok(plan),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Updates one plan-node model assignment using an observed revision.
    ///
    /// - `request`: Stable plan/node handles and safe assignment.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns a structured server or transport error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn set_plan_assignment(
        &self,
        request: SessionPlanAssignmentRequest,
    ) -> Result<SessionPlan, ClientError> {
        match self.send(Request::PlanAssignment(request))?.response {
            Response::Plan(plan) => Ok(plan),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Applies a complete optimistic plan revision.
    ///
    /// - `request`: New nodes, dependencies, replacements, and reason.
    ///
    /// The latest authoritative plan projection.
    ///
    /// Returns a structured server or transport error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn replan(&self, request: SessionPlanReplanRequest) -> Result<SessionPlan, ClientError> {
        match self.send(Request::PlanReplan(request))?.response {
            Response::Plan(plan) => Ok(plan),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Retries one failed plan node at an observed plan revision.
    ///
    /// - `request`: Session, plan, node, and expected revision.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns a structured server or transport error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn retry_plan_node(
        &self,
        request: SessionPlanRetryRequest,
    ) -> Result<SessionPlan, ClientError> {
        match self.send(Request::PlanRetry(request))?.response {
            Response::Plan(plan) => Ok(plan),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads one bounded page of a node's exact-snapshot attempt history.
    ///
    /// - `request`: Session, plan, node, snapshot, and page range.
    ///
    /// Attempt outcomes ordered by claim generation.
    /// # Returns
    /// The requested page.
    /// # Errors
    /// Returns a structured server, transport, or response-mismatch error.
    pub fn plan_attempt_history(
        &self,
        request: SessionPlanAttemptHistoryRequest,
    ) -> Result<SessionPlanAttemptHistoryPage, ClientError> {
        match self.send(Request::PlanAttemptHistory(request))?.response {
            Response::PlanAttemptHistory(page) => Ok(page),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Adds ordinary work to a running plan at its observed revision.
    ///
    /// - `request`: Session, plan, revision, and the next work item.
    ///
    /// The authoritative updated plan projection.
    ///
    /// Returns a structured server or transport error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn add_plan_next_step(
        &self,
        request: SessionPlanNextStepRequest,
    ) -> Result<SessionPlan, ClientError> {
        match self.send(Request::PlanNextStep(request))?.response {
            Response::Plan(plan) => Ok(plan),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Decomposes one plan node into structural child work.
    ///
    /// - `request`: Parent, revision, children, and completion requirement.
    ///
    /// The authoritative updated plan projection.
    ///
    /// Returns a structured server or transport error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn decompose_plan_node(
        &self,
        request: SessionPlanDecomposeRequest,
    ) -> Result<SessionPlan, ClientError> {
        match self.send(Request::PlanDecompose(request))?.response {
            Response::Plan(plan) => Ok(plan),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Applies one typed summary, evidence, gate, or waiver mutation.
    ///
    /// - `request`: Exact plan revision and semantic mutation.
    ///
    /// The authoritative updated plan projection.
    /// # Returns
    /// The updated plan.
    /// # Errors
    /// Returns a structured server or transport error.
    pub fn mutate_plan_semantics(
        &self,
        request: SessionPlanSemanticMutationRequest,
    ) -> Result<SessionPlan, ClientError> {
        match self.send(Request::PlanSemanticMutation(request))?.response {
            Response::Plan(plan) => Ok(plan),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads shared presentation preferences for the authenticated principal.
    ///
    /// - `self`: Client using the protocol transport.
    ///
    /// Revisioned shared presentation defaults.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn agent_presentation_preferences(
        &self,
    ) -> Result<AgentPresentationPreferencesResponse, ClientError> {
        match self
            .send(Request::AgentPresentationPreferencesRead(
                AgentPresentationPreferencesReadRequest {},
            ))?
            .response
        {
            Response::AgentPresentationPreferences(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Replaces shared preferences at the caller's observed revision.
    ///
    /// - `self`: Client using the protocol transport.
    /// - `expected_revision`: Revision from the last preference read.
    /// - `preferences`: Complete set of shared defaults.
    ///
    /// Updated revisioned shared presentation defaults.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn set_agent_presentation_preferences(
        &self,
        expected_revision: u64,
        preferences: PublicAgentPresentationPreferences,
    ) -> Result<AgentPresentationPreferencesResponse, ClientError> {
        match self
            .send(Request::AgentPresentationPreferencesSet(
                AgentPresentationPreferencesSetRequest {
                    expected_revision,
                    preferences,
                },
            ))?
            .response
        {
            Response::AgentPresentationPreferences(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads one branch and its nested descendants on demand.
    ///
    /// - `self`: Client using the protocol transport.
    /// - `session`: Stable session identity.
    /// - `branch`: Stable branch identity whose activity is requested.
    ///
    /// The selected branch subtree without unrelated branch histories.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn read_session_branch(
        &self,
        session: impl Into<String>,
        branch: impl Into<String>,
    ) -> Result<SessionReadResponse, ClientError> {
        match self
            .send(Request::SessionRead(SessionReadRequest {
                session: SessionReference { id: session.into() },
                branch: Some(BranchReference { id: branch.into() }),
            }))?
            .response
        {
            Response::Session(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Loads the captured safe model request for one invocation.
    ///
    /// - `self`: Client using the protocol transport.
    /// - `session`: Stable session identity.
    /// - `branch`: Stable branch identity.
    /// - `invocation`: Model invocation activity identity.
    ///
    /// Bounded provider-neutral request detail retained by the session.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn model_request_detail(
        &self,
        session: impl Into<String>,
        branch: impl Into<String>,
        invocation: impl Into<String>,
    ) -> Result<ModelRequestDetailResponse, ClientError> {
        match self
            .send(Request::ModelRequestDetail(ModelRequestDetailRequest {
                session: SessionReference { id: session.into() },
                branch: BranchReference { id: branch.into() },
                invocation: invocation.into(),
                offset: None,
                max_chars: None,
            }))?
            .response
        {
            Response::ModelRequestDetail(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Reads one explicitly bounded page of complete safe model request text.
    ///
    /// - `session`: Stable session identity.
    /// - `branch`: Stable branch identity.
    /// - `invocation`: Stable model-invocation identity.
    /// - `offset`: Unicode-scalar offset in the ordered text stream.
    /// - `max_chars`: Requested character bound, capped by the server.
    ///
    /// Request metadata and one text page.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn model_request_detail_page(
        &self,
        session: impl Into<String>,
        branch: impl Into<String>,
        invocation: impl Into<String>,
        offset: u64,
        max_chars: u32,
    ) -> Result<ModelRequestDetailResponse, ClientError> {
        match self
            .send(Request::ModelRequestDetail(ModelRequestDetailRequest {
                session: SessionReference { id: session.into() },
                branch: BranchReference { id: branch.into() },
                invocation: invocation.into(),
                offset: Some(offset),
                max_chars: Some(max_chars),
            }))?
            .response
        {
            Response::ModelRequestDetail(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }

    /// Loads bounded safe detail for one durable tool result.
    ///
    /// - `self`: Client using the protocol transport.
    /// - `session`: Stable session identity.
    /// - `branch`: Stable branch identity.
    /// - `result`: Tool result activity identity.
    ///
    /// Whitelisted safe result fields and a truncation indicator.
    ///
    /// Returns a structured server error or response mismatch.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn tool_result_detail(
        &self,
        session: impl Into<String>,
        branch: impl Into<String>,
        result: impl Into<String>,
    ) -> Result<ToolResultDetailResponse, ClientError> {
        match self
            .send(Request::ToolResultDetail(ToolResultDetailRequest {
                session: SessionReference { id: session.into() },
                branch: BranchReference { id: branch.into() },
                result: result.into(),
            }))?
            .response
        {
            Response::ToolResultDetail(response) => Ok(response),
            Response::Error(error) => Err(error.into()),
            _ => Err(ClientError::UnexpectedResponse),
        }
    }
}
