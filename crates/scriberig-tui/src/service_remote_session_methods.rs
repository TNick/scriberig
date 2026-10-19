//! Contains remote session read and submission contract methods.

macro_rules! remote_session_methods {
    () => {
        /// Reads one remote branch subtree on demand.
        ///
        /// - `_context`: Authorized application context.
        /// - `session`: Stable session identity.
        /// - `branch`: Branch whose activity is requested.
        ///
        /// The selected provider-neutral branch subtree.
        ///
        /// Returns a sanitized remote protocol failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn read_session_branch(
            &self,
            _context: &ApplicationContext,
            session: EntityId,
            branch: EntityId,
        ) -> Result<scriberig_session::Session, ApplicationError> {
            let response = self
                .client
                .read_session_branch(session.to_string(), branch.to_string())
                .map_err(remote_error)?;
            session_from_response(response)
        }

        /// Submits a remote agent command through the shared operation protocol.
        ///
        /// - `_context`: Authorized application context.
        /// - `session`: Stable session identity receiving the prompt.
        /// - `branch`: Optional target branch identity.
        /// - `prompt`: User-authored agent instruction.
        ///
        /// The accepted shared operation identity.
        ///
        /// Returns the sanitized remote protocol failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn submit_agent_command(
            &self,
            _context: &ApplicationContext,
            session: EntityId,
            branch: Option<EntityId>,
            prompt: &str,
        ) -> Result<scriberig_core::OperationId, ApplicationError> {
            super::service_remote_agent::submit_agent_command(self, session, branch, prompt)
        }

        /// Submits remote agent work while retaining the server-provided branch handle.
        ///
        /// - `_context`: Application context for the operation.
        /// - `session`: Stable session identity.
        /// - `branch`: Optional continuation branch.
        /// - `prompt`: User-authored instruction.
        ///
        /// Stable operation, session, and branch handles.
        ///
        /// Returns a sanitized remote protocol or identity error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn submit_agent_handle(
            &self,
            _context: &ApplicationContext,
            session: EntityId,
            branch: Option<EntityId>,
            prompt: &str,
        ) -> Result<super::AgentHandle, ApplicationError> {
            super::service_remote_agent::submit_agent_handle(self, session, branch, prompt)
        }
    };
}
