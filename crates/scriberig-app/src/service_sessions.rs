//! Provides application-owned fallback session and agent acceptance state.

use std::sync::atomic::Ordering;

use super::{ApplicationBackend, ApplicationService};
use crate::{ApplicationContext, ApplicationError};
use scriberig_core::EntityId;
use scriberig_session::Session;

#[path = "service_local_sessions.rs"]
mod service_local_sessions;

#[path = "service_sessions/agent_submission.rs"]
mod agent_submission;
#[path = "service_sessions/management.rs"]
mod management;
#[path = "service_sessions/plan_attempts.rs"]
mod plan_attempts;
#[macro_use]
#[path = "service_sessions/plan_node_details.rs"]
mod plan_node_details;
#[path = "service_sessions/plan_store.rs"]
mod plan_store;

#[path = "service_session_authorization.rs"]
mod service_session_authorization;
pub(super) use service_session_authorization::authorize_session_read;
pub(super) use service_session_authorization::{
    authorize_model_library_read, authorize_session_model_selection,
};

#[path = "service_agent_execution.rs"]
mod service_agent_execution;

impl<B: ApplicationBackend> crate::SessionBackend for ApplicationService<B> {
    /// Lists complete durable sessions through the application authorization boundary.
    ///
    /// - `self`: Shared application service.
    /// - `context`: Authorized caller context.
    ///
    /// Sessions ordered by stable identity.
    ///
    /// Returns authorization or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn list_sessions(
        &self,
        context: &ApplicationContext,
    ) -> Result<Vec<Session>, ApplicationError> {
        management::list_sessions(self, context)
    }

    /// Removes a durable session through the application authorization boundary.
    ///
    /// - `self`: Shared application service.
    /// - `context`: Authorized caller context.
    /// - `session`: Stable identity to remove.
    ///
    /// Unit after the exact session is removed.
    ///
    /// Returns authorization, active-work, or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn remove_session(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<(), ApplicationError> {
        management::remove_session(self, context, session)
    }

    /// Continues at an exact anchor through the shared application operation.
    ///
    /// - `self`: Shared application service.
    /// - `context`: Authorized request context.
    /// - `request`: Exact continuation location and user input.
    ///
    /// Authoritative operation, session, and continuation branch handles.
    ///
    /// Returns anchor validation, authorization, persistence, or runtime errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn continue_agent_at(
        &self,
        context: &ApplicationContext,
        request: crate::AgentContinueAtRequest,
    ) -> Result<crate::AgentContinuationAccepted, ApplicationError> {
        ApplicationService::continue_agent_at_request(self, context, request)
    }

    /// Starts a new session with a durable first human turn.
    ///
    /// - `self`: Shared application service.
    /// - `context`: Authorized request context.
    /// - `request`: Initial user input and optional semantic selection.
    ///
    /// Authoritative operation, session, and root branch handles.
    ///
    /// Returns validation, authorization, persistence, or runtime errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn start_new_session_agent(
        &self,
        context: &ApplicationContext,
        request: crate::NewSessionAgentRequest,
    ) -> Result<crate::NewSessionAgentStart, ApplicationError> {
        ApplicationService::start_new_session_agent_with_selection(self, context, request)
    }

    /// Delegates a session read to the backend or reads application-owned state.
    ///
    /// - `context`: Application context for the request.
    /// - `request`: Session request to resolve.
    ///
    /// The requested session.
    ///
    /// Returns the backend or session lookup error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn read_session(
        &self,
        context: &ApplicationContext,
        request: &crate::SessionRequest,
    ) -> Result<Session, ApplicationError> {
        authorize_session_read(context, request)?;
        match self.backend.read_session(context, request) {
            Ok(session) => Ok(session),
            Err(error) if error.category() == "unsupported_operation" => {
                let local = self.load_session_state(request.session)?;
                if let Some(session) = local {
                    return Ok(session);
                }
                Err(ApplicationError::NotFound {
                    subject: request.session.to_string(),
                })
            }
            Err(error) => Err(error),
        }
    }

    /// Reads a bounded page of node attempts through the authorized semantic store.
    ///
    /// - `context`: Authorized request context.
    /// - `request`: Node, exact snapshot, and page range to inspect.
    ///
    /// The requested attempts ordered by generation.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns authorization, invalid-page, history, or persistence failures.
    fn read_plan_attempt_history(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanAttemptHistoryRequest,
    ) -> Result<Vec<scriberig_session::PlanSemanticAttempt>, ApplicationError> {
        plan_attempts::read(self, context, request)
    }

    application_plan_node_details_method!();

    /// Persists a complete changed session through the backend or local store.
    ///
    /// - `context`: Authorized request context.
    /// - `session`: Complete changed session snapshot.
    ///
    /// Returns authorization or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn save_session(
        &self,
        context: &ApplicationContext,
        session: &Session,
    ) -> Result<(), ApplicationError> {
        service_session_authorization::authorize_session_mutation(context, session.id)?;
        match self.backend.save_session(context, session) {
            Ok(()) => Ok(()),
            Err(error) if error.category() == "unsupported_operation" => {
                self.save_session_state(session.clone())
            }
            Err(error) => Err(error),
        }
    }

    /// Atomically changes one plan at its observed revision.
    ///
    /// - `context`: Authorized request context.
    /// - `session`: Snapshot carrying the proposed plan.
    /// - `plan`: Stable plan identity.
    /// - `expected_revision`: Revision observed before mutation.
    ///
    /// Returns authorization, revision conflict, or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn compare_and_swap_plan(
        &self,
        context: &ApplicationContext,
        session: &Session,
        plan: EntityId,
        expected_revision: u64,
    ) -> Result<(), ApplicationError> {
        plan_store::compare_and_swap_plan(self, context, session, plan, expected_revision)
    }

    /// Creates a durable plan through the application plan service.
    ///
    /// - `context`: Authorized request context.
    /// - `request`: Goal, exact anchor, nodes, and dependencies.
    ///
    /// The authoritative persisted plan.
    ///
    /// Returns plan validation, authorization, or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn create_plan(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanCreateRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        ApplicationService::create_plan(self, context, request)
    }

    /// Applies a replan proposal through the shared application plan service.
    ///
    /// - `context`: Authorized request context.
    /// - `request`: Complete plan revision proposal.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns plan validation, conflict, authorization, or persistence errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn replan(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanReplanRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        ApplicationService::replan(self, context, request)
    }

    /// Applies one lifecycle action through the shared application service.
    ///
    /// - `context`: Authorized request context.
    /// - `request`: Plan and expected revision.
    /// - `action`: Requested transition.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns plan transition, revision conflict, authorization, or persistence errors.
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
        match action {
            crate::PlanLifecycleAction::Start => {
                ApplicationService::start_plan(self, context, request)
            }
            crate::PlanLifecycleAction::Pause => {
                ApplicationService::pause_plan(self, context, request)
            }
            crate::PlanLifecycleAction::Resume => {
                ApplicationService::resume_plan(self, context, request)
            }
            crate::PlanLifecycleAction::Stop => {
                ApplicationService::stop_plan(self, context, request)
            }
            crate::PlanLifecycleAction::Complete => {
                ApplicationService::complete_plan(self, context, request)
            }
        }
    }

    /// Retries a failed node through shared plan validation and persistence.
    /// - `context`: Authorized caller context.
    /// - `request`: Plan, node, and expected revision.
    /// # Returns
    /// The updated plan.
    /// # Errors
    /// Returns validation, conflict, or persistence errors.
    fn retry_plan_node(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanRetryRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        ApplicationService::retry_plan_node(self, context, request)
    }
    /// Adds ordinary work through the shared plan boundary.
    /// - `context`: Authorized caller context.
    /// - `request`: Plan revision and proposed next step.
    /// # Returns
    /// The updated plan.
    /// # Errors
    /// Returns validation, conflict, assignment, or persistence errors.
    fn add_plan_next_step(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanNextStepRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        ApplicationService::add_plan_next_step(self, context, request)
    }

    /// Decomposes one node through the shared plan boundary.
    /// - `context`: Authorized caller context.
    /// - `request`: Parent, revision, children, and requirement policy.
    /// # Returns
    /// The updated plan.
    /// # Errors
    /// Returns validation, conflict, assignment, or persistence errors.
    fn decompose_plan_node(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanDecomposeRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        ApplicationService::decompose_plan_node(self, context, request)
    }

    /// Applies one typed semantic mutation through the application service.
    ///
    /// - `context`: Authorized caller context.
    /// - `request`: Exact plan revision and semantic mutation.
    /// # Returns
    /// The updated plan.
    /// # Errors
    /// Returns validation, conflict, authorization, or persistence errors.
    fn mutate_plan_semantics(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanSemanticMutationRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        ApplicationService::mutate_plan_semantics(self, context, request)
    }

    /// Changes one node assignment through the application service.
    /// - `context`: Authorized request context.
    /// - `request`: Plan, node, revision, and safe assignment.
    /// # Returns
    /// The authoritative updated plan.
    /// # Errors
    /// Returns validation, authorization, or persistence errors.
    fn set_plan_node_assignment(
        &self,
        context: &ApplicationContext,
        request: &crate::PlanAssignmentRequest,
    ) -> Result<scriberig_session::Plan, ApplicationError> {
        ApplicationService::set_plan_node_assignment(self, context, request)
    }

    /// Delegates agent submission to the backend or accepts local session state.
    ///
    /// - `context`: Application context for the request.
    /// - `request`: Agent command request to submit.
    ///
    /// The accepted operation identity.
    ///
    /// Returns the backend or session state error.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn submit_agent_command(
        &self,
        context: &ApplicationContext,
        request: &crate::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        agent_submission::submit(self, context, request)
    }
}

impl<B> ApplicationService<B> {
    /// Allocates a deterministic semantic entity handle for local session state.
    ///
    /// - `super`: Application service.
    /// - `self`: Application service owning the handle sequence.
    /// - `marker`: Prefix byte distinguishing the handle family.
    ///
    /// A stable entity identifier unique within this service instance.
    /// # Returns
    /// The operation result.
    pub(super) fn next_entity(&self, marker: u8) -> EntityId {
        let sequence = self.next_agent_handle.fetch_add(1, Ordering::Relaxed);
        let mut bytes = [0_u8; 16];
        bytes[0] = marker;
        bytes[8..].copy_from_slice(&sequence.to_be_bytes());
        EntityId::from_bytes(bytes)
    }
}
