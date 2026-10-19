//! Contains application adapter handle resolution.

macro_rules! application_handle_method {
    () => {
        /// Submits local agent work and resolves the accepted root branch handle.
        ///
        /// - `context`: Application context for the operation.
        /// - `session`: Stable session identity.
        /// - `branch`: Optional continuation branch.
        /// - `prompt`: User-authored instruction.
        ///
        /// Stable operation, session, and branch handles.
        ///
        /// Returns the application submission or session-read error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn submit_agent_handle(
            &self,
            context: &ApplicationContext,
            session: EntityId,
            branch: Option<EntityId>,
            prompt: &str,
        ) -> Result<super::AgentHandle, ApplicationError> {
            let operation = self.submit_agent_command(context, session, branch, prompt)?;
            let branch = branch.or_else(|| {
                self.read_session(context, session)
                    .ok()
                    .and_then(|value| value.branches.last().map(|value| value.id))
            });
            Ok(super::AgentHandle {
                operation,
                session,
                branch,
                workspace: None,
            })
        }
    };
}
