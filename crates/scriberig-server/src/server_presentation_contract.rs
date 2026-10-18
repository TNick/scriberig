//! Defines default application boundary methods for shared display preferences.

macro_rules! agent_presentation_methods {
    () => {
        /// Reads shared principal-scoped agent presentation defaults.
        ///
        /// # Parameters
        /// - `_context`: Authorized caller context.
        ///
        /// # Returns
        /// Revisioned shared defaults independent from session history.
        ///
        /// # Errors
        /// Returns unsupported-operation when unavailable.
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
        /// # Parameters
        /// - `_context`: Authorized caller context.
        /// - `_request`: Expected revision and complete preference values.
        ///
        /// # Returns
        /// The updated shared defaults.
        ///
        /// # Errors
        /// Returns unsupported-operation when unavailable.
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
