//! Contains remaining operation adapter contract methods.

macro_rules! tui_operation_contract_methods {
    () => {
        /// Lists current shared application operation records.
        ///
        /// - `_context`: Application context for operation visibility.
        ///
        /// Current application operation records.
        ///
        /// This local adapter has no separate listing failure path.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn operation_records(
            &self,
            _context: &ApplicationContext,
        ) -> Result<Vec<scriberig_app::OperationRecord>, ApplicationError> {
            Ok(self.service.operations().list())
        }

        /// Returns lifecycle events retained by the shared application registry.
        ///
        /// - `_context`: Application context for event access.
        ///
        /// Retained application events.
        ///
        /// This adapter does not return an event access error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn application_events(
            &self,
            _context: &ApplicationContext,
        ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
            Ok(service_operation_helpers::application_events(self))
        }

        /// Reads a session through the shared application boundary.
        ///
        /// - `context`: Application context for the request.
        /// - `session`: Session identity to read.
        ///
        /// The requested session.
        ///
        /// Returns the application session error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn read_session(
            &self,
            context: &ApplicationContext,
            session: EntityId,
        ) -> Result<scriberig_session::Session, ApplicationError> {
            service_operation_helpers::read_session(self, context, session)
        }

        /// Lists sessions through the shared application session repository.
        ///
        /// - `context`: Caller authorization context.
        ///
        /// Sessions ordered by stable identity.
        ///
        /// Returns an authorization or persistence failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when listing fails.
        fn list_sessions(
            &self,
            context: &ApplicationContext,
        ) -> Result<Vec<scriberig_session::Session>, ApplicationError> {
            scriberig_app::SessionBackend::list_sessions(&self.service, context)
        }

        /// Removes one session through the shared application boundary.
        ///
        /// - `context`: Caller authorization context.
        /// - `session`: Exact session identity to remove.
        ///
        /// Unit after the durable session is removed.
        ///
        /// Returns an authorization, active-work, or persistence failure.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when removal fails.
        fn remove_session(
            &self,
            context: &ApplicationContext,
            session: EntityId,
        ) -> Result<(), ApplicationError> {
            scriberig_app::SessionBackend::remove_session(&self.service, context, session)
        }

        /// Submits an agent command through the shared application boundary.
        ///
        /// - `context`: Application context for the request.
        /// - `session`: Session identity receiving the prompt.
        /// - `branch`: Optional branch identity.
        /// - `prompt`: Agent prompt to submit.
        ///
        /// The accepted operation identity.
        ///
        /// Returns the application agent error.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn submit_agent_command(
            &self,
            context: &ApplicationContext,
            session: EntityId,
            branch: Option<EntityId>,
            prompt: &str,
        ) -> Result<scriberig_core::OperationId, ApplicationError> {
            service_operation_helpers::submit_agent_command(self, context, session, branch, prompt)
        }
    };
}
