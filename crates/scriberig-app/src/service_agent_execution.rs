//! Owns application entry points for composed local agent execution.

use super::ApplicationService;
use super::service_session_authorization::authorize_agent_command;
use crate::{ApplicationContext, ApplicationError};

impl<B> ApplicationService<B> {
    /// Starts a new local session and leaves its operation active for a runtime.
    ///
    /// - `self`: Application service owning local session and operation state.
    /// - `context`: Caller capabilities and initial semantic scope.
    /// - `request`: First user input and optional semantic selection.
    ///
    /// The pending operation, newly allocated session, and root branch handles.
    ///
    /// Returns authorization, session persistence, or operation errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn begin_new_session_agent(
        &self,
        context: &ApplicationContext,
        request: crate::NewSessionAgentRequest,
    ) -> Result<crate::LocalAgentHandle, ApplicationError> {
        let session = self.next_entity(0xA0);
        self.begin_agent_execution(
            context,
            &crate::AgentCommandRequest {
                session,
                branch: None,
                anchor: None,
                prompt: request.prompt,
                selection: request.selection,
            },
        )
    }

    /// Begins an exact-anchor continuation and leaves it pending for the runtime.
    ///
    /// - `self`: Application service owning session and operation state.
    /// - `context`: Caller capabilities and semantic scope.
    /// - `request`: Session, exact reply anchor, prompt, and optional selection.
    ///
    /// The pending operation, session, and continuation branch handles.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when authorization, anchor validation, or persistence fails.
    pub fn begin_continue_agent_at(
        &self,
        context: &ApplicationContext,
        request: crate::AgentContinueAtRequest,
    ) -> Result<crate::LocalAgentHandle, ApplicationError> {
        self.begin_agent_execution(
            context,
            &crate::AgentCommandRequest {
                session: request.session,
                branch: None,
                anchor: Some(request.anchor),
                prompt: request.prompt,
                selection: request.selection,
            },
        )
    }

    /// Begins a local agent operation after checking its public capability.
    ///
    /// The returned operation remains running so a composed provider runtime can
    /// execute it and store the result through `complete_local_agent`.
    ///
    /// - `self`: Application service owning session and operation state.
    /// - `context`: Caller capabilities and exact request context.
    /// - `request`: Session, branch, and user instruction.
    ///
    /// The pending operation, session, and branch handles.
    ///
    /// Returns an authorization or session-state failure.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    pub fn begin_agent_execution(
        &self,
        context: &ApplicationContext,
        request: &crate::AgentCommandRequest,
    ) -> Result<crate::LocalAgentHandle, ApplicationError> {
        authorize_agent_command(context, request)?;
        self.start_local_agent(request)
    }
}
