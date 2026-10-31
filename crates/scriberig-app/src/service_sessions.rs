//! Provides application-owned fallback session and agent acceptance state.

use std::sync::atomic::Ordering;

use super::{ApplicationBackend, ApplicationService};
use crate::{ApplicationContext, ApplicationError};
use scriberig_core::EntityId;
use scriberig_session::Session;

#[path = "service_local_sessions.rs"]
pub(crate) mod service_local_sessions;

#[path = "service_sessions/management.rs"]
#[macro_use]
mod management;
#[path = "service_sessions/plan_attempts.rs"]
mod plan_attempts;
#[macro_use]
#[path = "service_sessions/plan_node_details.rs"]
mod plan_node_details;
#[path = "service_sessions/plan_store.rs"]
mod plan_store;
#[path = "service_sessions/plan_operations.rs"]
#[macro_use]
mod plan_operations;

#[path = "service_session_authorization.rs"]
mod service_session_authorization;
use service_session_authorization::authorize_agent_command;
pub(super) use service_session_authorization::{
    authorize_model_library_read, authorize_plan_read, authorize_session_model_selection,
    authorize_session_mutation, authorize_session_read,
};

/// Authorizes a compact plan read with the session-tree capability.
///
/// - `context`: Authorized caller context.
/// - `session`: Stable `ScribeRig` session identity.
///
/// Unit when the caller may inspect the session tree.
///
/// Returns permission-denied when the caller lacks read access.
/// # Returns
/// The authorization result.
/// # Errors
/// Returns an authorization construction or permission error.
#[path = "service_agent_execution.rs"]
mod service_agent_execution;

impl<B: ApplicationBackend> crate::SessionBackend for ApplicationService<B> {
    service_session_management_methods!();

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

    /// Reads a compact projection for routine session tree responses.
    ///
    /// - `context`: Application context for the request.
    /// - `request`: Stable session identity to resolve.
    ///
    /// The session projection with compact semantic plan state when storage
    /// supports it.
    ///
    /// # Returns
    /// The authorized session projection.
    ///
    /// # Errors
    /// Returns an authorization, lookup, or persistence error.
    fn read_session_projection(
        &self,
        context: &ApplicationContext,
        request: &crate::SessionRequest,
    ) -> Result<Session, ApplicationError> {
        authorize_session_read(context, request)?;
        match self.backend.read_session_projection(context, request) {
            Ok(session) => Ok(session),
            Err(error) if error.category() == "unsupported_operation" => self
                .load_session_read_projection_state(request.session)?
                .ok_or_else(|| ApplicationError::NotFound {
                    subject: request.session.to_string(),
                }),
            Err(error) => Err(error),
        }
    }

    /// Reads a bounded exact-snapshot plan-attempt page through the repository.
    ///
    /// # Parameters
    /// - `context`: Authorized request context.
    /// - `request`: Node, snapshot, and page range to inspect.
    ///
    /// # Returns
    /// The transcript-free attempt page.
    ///
    /// # Errors
    /// Returns authorization, limit, history, or persistence failures.
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
        let mut compacted = session.clone();
        compacted.compact_branch_results();
        match self.backend.save_session(context, &compacted) {
            Ok(()) => Ok(()),
            Err(error) if error.category() == "unsupported_operation" => {
                self.save_session_state(compacted)
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

    application_plan_operation_methods!();

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
        authorize_agent_command(context, request)?;
        if let Some(anchor) = request.anchor {
            if request.branch.is_some() {
                return Err(ApplicationError::InvalidRequest {
                    operation: "agent_continue_at".to_owned(),
                    message: "an exact anchor cannot be combined with a branch-head target"
                        .to_owned(),
                });
            }
            let session = self.read_session(
                context,
                &crate::SessionRequest {
                    session: request.session,
                },
            )?;
            session
                .resolve_anchor(anchor)
                .map_err(|error| ApplicationError::InvalidRequest {
                    operation: "agent_continue_at".to_owned(),
                    message: error.to_string(),
                })?;
        }
        let operation = match self.backend.submit_agent_command(context, request) {
            Ok(operation) => operation,
            Err(error) if error.category() == "unsupported_operation" => {
                self.accept_local_agent(request)?
            }
            Err(error) => return Err(error),
        };
        if self.operations.get(&operation).is_err() {
            self.operations
                .start(operation.clone(), crate::OperationKind::Agent)?;
            self.operations
                .transition(&operation, crate::OperationStatus::Running)?;
        }
        Ok(operation)
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
