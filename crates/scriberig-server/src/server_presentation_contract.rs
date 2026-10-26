//! Defines default application boundary methods for shared display reads and
//! the model-catalog generation those display rows come from.

macro_rules! agent_presentation_methods {
    () => {
        /// Names the upstream model catalog generation this service reads from.
        ///
        /// A host credential-readiness projection carries the catalog generation
        /// it resolved, and the transport only pairs that projection with model
        /// rows read from the same generation.
        ///
        /// The current upstream catalog generation, or `None` when this service
        /// has no upstream model catalog to bind display rows to.
        /// # Returns
        /// The generation current model-library rows were projected from.
        fn model_catalog_generation(&self) -> Option<scriberig_app::ModelCatalogGeneration> {
            None
        }

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
