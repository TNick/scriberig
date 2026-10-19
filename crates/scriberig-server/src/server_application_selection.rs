//! Defines shared model and thinking selection adapter methods.

macro_rules! server_execution_selection_methods {
    () => {
        /// Selects a model profile for future session invocations.
        ///
        /// - `context`: Trusted capability context.
        /// - `session`: Stable session identity.
        /// - `expected_revision`: Selection revision observed by the caller.
        /// - `model_profile`: Curated profile identity.
        ///
        /// Updated session execution selection.
        ///
        /// Returns authorization, validation, conflict, or persistence failures.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_model(
            &self,
            context: &ApplicationContext,
            session: &str,
            expected_revision: u64,
            model_profile: &str,
        ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
            server_agent_console::set_session_model(
                self,
                context,
                session,
                expected_revision,
                model_profile,
            )
        }

        /// Selects a thinking level for future session invocations.
        ///
        /// - `context`: Trusted capability context.
        /// - `session`: Stable session identity.
        /// - `expected_revision`: Selection revision observed by the caller.
        /// - `thinking_level`: Advertised thinking capability identity.
        ///
        /// Updated session execution selection.
        ///
        /// Returns authorization, validation, conflict, or persistence failures.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_session_thinking_level(
            &self,
            context: &ApplicationContext,
            session: &str,
            expected_revision: u64,
            thinking_level: &str,
        ) -> Result<scriberig_protocol::SessionExecutionSelection, ApplicationError> {
            server_agent_console::set_session_thinking_level(
                self,
                context,
                session,
                expected_revision,
                thinking_level,
            )
        }
    };
}
