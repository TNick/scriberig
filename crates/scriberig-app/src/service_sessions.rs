//! Provides application-owned fallback session and agent acceptance state.

use std::sync::atomic::Ordering;

use super::{ApplicationBackend, ApplicationService};
use crate::{ApplicationContext, ApplicationError};
use scriberig_core::EntityId;
use scriberig_session::Session;

#[path = "service_local_sessions.rs"]
mod service_local_sessions;

#[path = "service_session_authorization.rs"]
mod service_session_authorization;
use service_session_authorization::{authorize_agent_command, authorize_session_read};
pub(super) use service_session_authorization::{
    authorize_model_library_read, authorize_session_model_selection,
};

#[path = "service_agent_execution.rs"]
mod service_agent_execution;

impl<B: ApplicationBackend> crate::SessionBackend for ApplicationService<B> {
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
        service_session_authorization::authorize_session_mutation(context, session.id)?;
        if let Some(store) = &self.session_store {
            return store
                .compare_and_swap_plan(session, plan, expected_revision)
                .map_err(map_plan_store_error);
        }
        match self
            .backend
            .compare_and_swap_plan(context, session, plan, expected_revision)
        {
            Ok(()) => return Ok(()),
            Err(error) if error.category() != "unsupported_operation" => return Err(error),
            Err(_) => {}
        }
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| ApplicationError::Internal {
                message: "application session lock failed".to_owned(),
            })?;
        let stored = sessions
            .get_mut(&session.id)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: session.id.to_string(),
            })?;
        let proposed_plan = session
            .plans
            .iter()
            .find(|entry| entry.id == plan)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: plan.to_string(),
            })?;
        let stored_plan = stored
            .plans
            .iter_mut()
            .find(|entry| entry.id == plan)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: plan.to_string(),
            })?;
        if stored_plan.revision != expected_revision {
            return Err(map_plan_store_error(
                scriberig_session::PlanStoreError::RevisionConflict {
                    expected: expected_revision,
                    actual: stored_plan.revision,
                },
            ));
        }
        if proposed_plan.revision != expected_revision.saturating_add(1) {
            return Err(ApplicationError::Internal {
                message: "proposed plan has an unexpected revision".to_owned(),
            });
        }
        *stored_plan = proposed_plan.clone();
        for branch in &session.branches {
            if stored.branches.iter().any(|known| known.id == branch.id) {
                continue;
            }
            stored.branches.push(branch.clone());
            if let Some(parent_id) = branch.parent
                && let Some(parent) = stored
                    .branches
                    .iter_mut()
                    .find(|known| known.id == parent_id)
                && !parent.children.contains(&branch.id)
            {
                parent.children.push(branch.id);
            }
        }
        Ok(())
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

    /// Writes one validated node assignment through the application service.
    ///
    /// - `context`: Authorized request context.
    /// - `request`: Plan, node, revision, and credential-safe assignment.
    ///
    /// The authoritative updated plan.
    ///
    /// Returns plan validation, authorization, or persistence failures.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
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

/// Maps a session repository plan mutation failure into an application error.
///
/// - `error`: Typed repository failure.
///
/// A stable conflict or internal application failure.
/// # Returns
/// The operation result.
fn map_plan_store_error(error: scriberig_session::PlanStoreError) -> ApplicationError {
    match error {
        scriberig_session::PlanStoreError::RevisionConflict { expected, actual } => {
            ApplicationError::Conflict {
                subject: "plan revision".to_owned(),
                details: Box::new(crate::ConflictDetails {
                    operation: Some(format!("expected {expected}, actual {actual}")),
                    ..crate::ConflictDetails::default()
                }),
            }
        }
        scriberig_session::PlanStoreError::Persistence(message) => {
            ApplicationError::Internal { message }
        }
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
