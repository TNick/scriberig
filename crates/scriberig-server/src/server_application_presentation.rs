//! Defines server forwarding methods for shared presentation preferences.

macro_rules! agent_presentation_impl_methods {
    () => {
        /// Names the generation published by the service's shared upstream catalog.
        ///
        /// The value is upstream's own catalog revision, so the transport can
        /// detect a reload that happened while a model-library response was
        /// being assembled.
        /// # Returns
        /// The current upstream model catalog generation.
        fn model_catalog_generation(&self) -> Option<scriberig_app::ModelCatalogGeneration> {
            Some(scriberig_app::catalog_generation(&self.model_catalog()))
        }

        /// Reads safe model descriptors and preserves the legacy profiles
        /// field.
        ///
        /// - `context`: Trusted capability context.
        ///
        /// Credential-safe public model response with no local profiles.
        ///
        /// Returns authorization or configuration failures.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn model_library(
            &self,
            context: &ApplicationContext,
        ) -> Result<scriberig_protocol::ModelLibraryResponse, ApplicationError> {
            super::server_agent_console::model_library(self, context)
        }

        /// Reads shared principal-scoped agent presentation defaults.
        ///
        /// - `context`: Trusted capability and principal context.
        ///
        /// Revisioned shared defaults.
        ///
        /// Returns authorization or preference-store failures.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn agent_presentation_preferences(
            &self,
            context: &ApplicationContext,
        ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
            super::server_agent_preferences::read(self, context)
        }

        /// Replaces shared presentation defaults at an expected revision.
        ///
        /// - `context`: Trusted capability and principal context.
        /// - `request`: Expected revision and preference values.
        ///
        /// The updated shared defaults.
        ///
        /// Returns authorization, revision-conflict, or preference-store failures.
        /// # Returns
        /// The operation result.
        /// # Errors
        /// Returns an error when the operation fails.
        fn set_agent_presentation_preferences(
            &self,
            context: &ApplicationContext,
            request: &scriberig_protocol::AgentPresentationPreferencesSetRequest,
        ) -> Result<scriberig_protocol::AgentPresentationPreferencesResponse, ApplicationError> {
            super::server_agent_preferences::set(self, context, request)
        }
    };
}
