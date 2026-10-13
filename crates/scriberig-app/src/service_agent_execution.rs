//! Owns application entry points for composed local agent execution.

use super::ApplicationService;
use super::service_session_authorization::authorize_agent_command;
use crate::{ApplicationContext, ApplicationError};

impl<B> ApplicationService<B> {
    /// Begins a local agent operation after checking its public capability.
    ///
    /// The returned operation remains running so a composed provider runtime can
    /// execute it and store the result through `complete_local_agent`.
    ///
    /// # Parameters
    /// - `self`: Application service owning session and operation state.
    /// - `context`: Caller capabilities and exact request context.
    /// - `request`: Session, branch, and user instruction.
    ///
    /// # Returns
    /// The pending operation, session, and branch handles.
    ///
    /// # Errors
    /// Returns an authorization or session-state failure.
    pub fn begin_agent_execution(
        &self,
        context: &ApplicationContext,
        request: &crate::AgentCommandRequest,
    ) -> Result<crate::LocalAgentHandle, ApplicationError> {
        authorize_agent_command(context, request)?;
        self.start_local_agent(request)
    }
}
