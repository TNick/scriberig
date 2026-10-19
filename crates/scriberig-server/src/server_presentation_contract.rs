//! Defines default application boundary methods for shared display preferences.

macro_rules! agent_presentation_methods {
    () => {
        /// Reads shared principal-scoped agent presentation defaults.
        ///
        /// - `_context`: Authorized caller context.
        ///
        /// Revisioned shared defaults independent from session history.
        ///
        /// Returns unsupported-operation when unavailable.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn agent_presentation_preferences(
            &self,
            _context: &ApplicationContext,
        ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "agent presentation preferences".to_owned(),
            })
        }

        /// Updates shared presentation defaults with an expected revision.
        ///
        /// - `_context`: Authorized caller context.
        /// - `_request`: Expected revision and complete preference values.
        ///
        /// The updated shared defaults.
        ///
        /// Returns unsupported-operation when unavailable.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_agent_presentation_preferences(
            &self,
            _context: &ApplicationContext,
            _request: &scriberig_protocol::AgentPresentationPreferencesSetRequest,
        ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
            Err(ApplicationError::UnsupportedOperation {
                operation: "agent presentation preferences".to_owned(),
            })
        }
    };
}
