//! Defines server forwarding methods for shared presentation preferences.

macro_rules! agent_presentation_impl_methods {
    () => {
        /// Reads safe model descriptors and curated profiles.
        ///
        /// # Parameters
        /// - `context`: Trusted capability context.
        ///
        /// # Returns
        /// Credential-safe public model library.
        ///
        /// # Errors
        /// Returns authorization or configuration failures.
        fn model_library(
            &self,
            context: &ApplicationContext,
        ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
            super::server_agent_console::model_library(self, context)
        }

        /// Reads shared principal-scoped agent presentation defaults.
        ///
        /// # Parameters
        /// - `context`: Trusted capability and principal context.
        ///
        /// # Returns
        /// Revisioned shared defaults.
        ///
        /// # Errors
        /// Returns authorization or preference-store failures.
        fn agent_presentation_preferences(
            &self,
            context: &ApplicationContext,
        ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
            super::server_agent_preferences::read(self, context)
        }

        /// Replaces shared presentation defaults at an expected revision.
        ///
        /// # Parameters
        /// - `context`: Trusted capability and principal context.
        /// - `request`: Expected revision and preference values.
        ///
        /// # Returns
        /// The updated shared defaults.
        ///
        /// # Errors
        /// Returns authorization, revision-conflict, or preference-store failures.
        fn set_agent_presentation_preferences(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::AgentPresentationPreferencesSetRequest,
        ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
            super::server_agent_preferences::set(self, context, request)
        }
    };
}
