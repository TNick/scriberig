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
    /// Delegates a session read to the backend or reads application-owned state.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Session request to resolve.
    ///
    /// # Returns
    /// The requested session.
    ///
    /// # Errors
    /// Returns the backend or session lookup error.
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

    /// Delegates agent submission to the backend or accepts local session state.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Agent command request to submit.
    ///
    /// # Returns
    /// The accepted operation identity.
    ///
    /// # Errors
    /// Returns the backend or session state error.
    fn submit_agent_command(
        &self,
        context: &ApplicationContext,
        request: &crate::AgentCommandRequest,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        authorize_agent_command(context, request)?;
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
    /// # Parameters
    /// - `super`: Application service.
    /// - `self`: Application service owning the handle sequence.
    /// - `marker`: Prefix byte distinguishing the handle family.
    ///
    /// # Returns
    /// A stable entity identifier unique within this service instance.
    pub(super) fn next_entity(&self, marker: u8) -> EntityId {
        let sequence = self.next_agent_handle.fetch_add(1, Ordering::Relaxed);
        let mut bytes = [0_u8; 16];
        bytes[0] = marker;
        bytes[8..].copy_from_slice(&sequence.to_be_bytes());
        EntityId::from_bytes(bytes)
    }
}
